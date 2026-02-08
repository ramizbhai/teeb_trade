use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde::Deserialize;
use url::Url;
use crate::model::{MarketData, SymbolState};
use crate::store::SharedState;
use crate::scanner::{check_for_signals, WsMessage};
use log::{info, error};
// using crate::verifier inside function

// Kline Event from !kline_1m without @arr effectively?
// Actually for All Market Mini Tickers it is !miniTicker@arr.
// For Klines, generic stream is <symbol>@kline_<interval>.
// There is NO !kline@arr for all symbols.
// Wait, is there?
// Documentation says: "Aggregate Trade Streams", "Mark Price", "Ticker".
// No "!kline@arr".
// WE MUST USE !miniticker or !ticker.
//
// Re-read user request: "Binance WebSocket se !ticker@arr ya !kline_1m@arr ka data fetch karna hai."
// The user MIGHT be mistaken about `!kline_1m@arr` existing.
// Let's assume `!ticker@arr` is the intended one if `!kline` doesn't exist for all.
//
// BUT `!bookTicker` exists.
//
// Let's go back to `!ticker@arr` (24h rolling).
// And use the "Volume Delta" approach.
// 
// To make it robust:
// Store `last_volume` and `last_timestamp`.
// `delta_vol = current_vol - last_vol`.
// If `delta_vol < 0`, `delta_vol = current_vol` (new day).
// Push `delta_vol` to a minute accumulator.
//
// Actually, I'll stick to the "Snapshot every minute" approach for the Sliding Window (History).
// But for the "Current Spike", I can compare `Current 24h Vol` to `Volume 1 min ago`?
// No, that requires history.
//
// Let's try to find if there's a way to get all klines.
// If not, I will implement the "Snapshot 24h Volume" every minute.
// This means the "Current Candle" is built by `current_24h - 24h_at_start_of_minute`.
//
// Let's implement `binance_client.rs` to handle `!ticker@arr` and track the start-of-minute volume.

#[derive(Debug, Deserialize)]
struct TickerEvent {
    s: String, // Symbol
    c: String, // Close price
    v: String, // Total traded base asset volume
    E: i64,    // Event time
}

// We need a map to store "Volume at start of current minute" for each symbol.
// And "Last updated minute timestamp".

pub async fn binance_ws_task(store: SharedState, tx: tokio::sync::broadcast::Sender<WsMessage>) {
    let url = Url::parse("wss://fstream.binance.com/ws/!ticker@arr").unwrap();
    info!("Connecting to Binance WebSocket: {}", url);

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    info!("Connected to Binance WebSocket");

    let (_, mut read) = ws_stream.split();
    
    // We need a local map to track volume at the start of the minute to calculate "current minute volume".
    // Map<Symbol, (StartOfMinuteVolume, MinuteTimestamp)>
    let mut volume_cache: dashmap::DashMap<String, (f64, i64)> = dashmap::DashMap::new();
    let mut last_update_broadcast: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(events) = serde_json::from_str::<Vec<TickerEvent>>(&text) {
                    for event in events {
                        let symbol = event.s;
                        let price = event.c.parse::<f64>().unwrap_or(0.0);
                        let volume_total = event.v.parse::<f64>().unwrap_or(0.0);
                        let event_time = event.E;
                        
                        // Round to minute
                        let current_minute = event_time / 60000;
                        
                        // Get or Insert cache
                        let mut cache_entry = volume_cache.entry(symbol.clone()).or_insert((volume_total, current_minute));
                        
                        if cache_entry.1 < current_minute {
                            // New minute started!
                            // 1. Finalize the previous candle and push to History
                            let prev_vol_total = cache_entry.0;
                            let prev_minute_vol = if volume_total >= prev_vol_total {
                                volume_total - prev_vol_total
                            } else {
                                volume_total // Reset happened
                            };
                            
                            let mut state_entry = store.entry(symbol.clone()).or_insert_with(|| SymbolState { 
                                symbol: symbol.clone(), 
                                window: std::collections::VecDeque::new(),
                                last_signal_time: None,
                            });
                            
                            // Push to window
                            state_entry.add_data(MarketData {
                                symbol: symbol.clone(),
                                price,
                                volume: prev_minute_vol,
                                timestamp: event_time,
                            });
                            
                            // 2. Reset cache for new minute
                            cache_entry.0 = volume_total;
                            cache_entry.1 = current_minute;
                        } else {
                            // Same minute. 
                            // Calculate "Current Minute Volume" so far.
                            let start_of_min_vol = cache_entry.0;
                            let current_min_vol = if volume_total >= start_of_min_vol {
                                volume_total - start_of_min_vol
                            } else {
                                volume_total
                            };
                            
                            let start_of_min_vol = cache_entry.0;
                            // Check Signaler immediately! (Real-time)
                            
                            // 1. Prepare Market Data
                            let market_data = MarketData {
                                symbol: symbol.clone(),
                                price,
                                volume: current_min_vol,
                                timestamp: event_time,
                            };

                            // 2. Check Signals
                            let mut signal_found = None;
                            if let Some(state_entry) = store.get(&symbol) {
                                if let Some(signal) = check_for_signals(&state_entry, &market_data, 0.0) {
                                     signal_found = Some(signal);
                                } else {
                                    // Check for "Live Update" if active signal exists within 60 mins
                                    if let Some(last_time) = state_entry.last_signal_time {
                                        if event_time - last_time < 60 * 60 * 1000 {
                                            // THROTTLE: Only update every 2000ms
                                            let last_broadcast = last_update_broadcast.get(&symbol).cloned().unwrap_or(0);
                                            if event_time - last_broadcast > 2000 {
                                                // Broadcast Update
                                                let update = crate::scanner::SignalUpdate {
                                                    symbol: symbol.clone(),
                                                    price: market_data.price,
                                                    volume: market_data.volume,
                                                    timestamp: market_data.timestamp,
                                                };
                                                if let Ok(_) = tx.send(crate::scanner::WsMessage::Update(update)) {
                                                    last_update_broadcast.insert(symbol.clone(), event_time);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // 3. Process Signal (Outside lock)
                            if let Some(mut signal) = signal_found {
                                // Update Last Signal Time
                                if let Some(mut state_mut) = store.get_mut(&symbol) {
                                     state_mut.last_signal_time = Some(market_data.timestamp);
                                }
                                
                                let tx = tx.clone();
                                tokio::spawn(async move {
                                    if crate::verifier::verify_signal(&mut signal).await {
                                        let _ = tx.send(crate::scanner::WsMessage::Signal(signal));
                                    }
                                });
                            }
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => error!("WS Error: {:?}", e),
        }
    }
}
