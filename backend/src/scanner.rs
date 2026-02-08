use crate::model::{MarketData, SymbolState};
use serde::{Deserialize, Serialize};
use log::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalType {
    Long,
    Short,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub symbol: String,
    pub signal_type: SignalType,
    pub price: f64,
    pub volume: f64,
    pub avg_volume: f64,
    pub timestamp: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalUpdate {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")] // "type": "signal", "payload": { ... }
pub enum WsMessage {
    Signal(Signal),
    Update(SignalUpdate),
    History(Vec<Signal>),
    Stats(crate::history::Stats), 
}

pub fn check_for_signals(state: &SymbolState, current_data: &MarketData, taker_buy_vol: f64) -> Option<Signal> {
    let avg_vol = state.get_average_volume();
    
    // Silent Watcher: Filter out absolute dust, but keep low-cap "dead" coins.
    // measurable "activity" usually means at least some value traded.
    // Let's say min 10k USDT volume to be significant for a "Whale".
    // Or maybe 50k? Let's stick to 10k for now to catch early moves.
    // Note: current_data.volume is in Base Asset? No, `!ticker` 'v' is Base Asset Volume.
    // We need Quote Asset Volume 'q' (or 'V' in ticker) for USDT value.
    // In our model `MarketData`, `volume` is whatever we passed.
    // In `binance_client.rs`, we parsed 'v' (Base Asset).
    // So Value = Volume * Price.
    
    let current_value = current_data.volume * current_data.price;
    let avg_value = avg_vol * current_data.price;

    if current_value < 10_000.0 {
        return None;
    }

    let volume_ratio = if avg_vol > 0.0 { current_data.volume / avg_vol } else { 0.0 };
    
    // Logic Refinement:
    // 1. Min 24h Volume (Actually avg_value of window is small for low vol coins)
    //    We want coins with substantial volume. Let's filter avg_value > $50k
    if avg_value < 50_000.0 {
        return None;
    }

    // 2. Cooldown Check (30 mins = 1800s * 1000ms)
    if let Some(last_time) = state.last_signal_time {
        if current_data.timestamp - last_time < 30 * 60 * 1000 {
            return None;
        }
    }
    
    let last_close = state.window.back().map(|d| d.price).unwrap_or(current_data.price);
    let price_change_percent = (current_data.price - last_close).abs() / last_close;

    // Logic: 
    // 1. "Dead" Coin waking up: Avg Value < 100k (Dead) AND Vol > 5x Avg. -> But we filter < 50k. So 50k-100k range.
    // 2. Active Coin spike: Vol > 3x Avg.
    
    let is_dead_wakeup = avg_value < 100_000.0 && volume_ratio > 5.0;
    let is_normal_spike = volume_ratio > 3.0;

    if (is_dead_wakeup || is_normal_spike) && price_change_percent < 0.008 {
         // Determine direction
        let taker_sell_vol = current_data.volume - taker_buy_vol;
        
        let signal_type = if taker_buy_vol > taker_sell_vol {
            SignalType::Long
        } else {
            SignalType::Short
        };

        let current_value = current_data.volume * current_data.price; // Re-calculate for log if needed, or stick to prev variable
        
        info!("Silent Watcher Detected: {:?} for {} (Val: ${:.0}, Ratio: {:.1}x, Price Chg: {:.4}%)", 
              signal_type, current_data.symbol, current_value, volume_ratio, price_change_percent*100.0);

        return Some(Signal {
            symbol: current_data.symbol.clone(),
            signal_type,
            price: current_data.price,
            volume: current_data.volume,
            avg_volume: avg_vol,
            timestamp: current_data.timestamp,
            reason: format!("Silent Alert! Vol: {:.1}x (Avg ${:.0}k), Price stable ({:.2}%)", volume_ratio, avg_value/1000.0, price_change_percent*100.0),
        });
    }

    None
}
