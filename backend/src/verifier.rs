use crate::scanner::{Signal, SignalType};
use reqwest::Client;
use serde::Deserialize;
use log::{info, warn};

#[derive(Debug, Deserialize)]
struct Depth {
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

// Open Interest Response
#[derive(Debug, Deserialize)]
struct OpenInterest {
    symbol: String,
    openInterest: String,
    time: i64,
}

pub async fn verify_signal(signal: &mut Signal) -> bool {
    let client = Client::new();
    
    // 1. Check Order Book Depth
    // API: https://fapi.binance.com/fapi/v1/depth?symbol=BTCUSDT&limit=20
    let depth_url = format!("https://fapi.binance.com/fapi/v1/depth?symbol={}&limit=20", signal.symbol);
    
    match client.get(&depth_url).send().await {
        Ok(resp) => {
            if let Ok(depth) = resp.json::<Depth>().await {
                let bid_wall = calculate_wall(depth.bids);
                let ask_wall = calculate_wall(depth.asks);
                
                info!("Order Book for {}: Bid Wall: {:.2}, Ask Wall: {:.2}", signal.symbol, bid_wall, ask_wall);
                
                match signal.signal_type {
                    SignalType::Long => {
                        let ratio = if ask_wall > 0.0 { bid_wall / ask_wall } else { 0.0 };
                        if ratio > 1.2 {
                            signal.reason += &format!(" | Strong Buy Wall (x{:.1})", ratio);
                        } else {
                             signal.reason += &format!(" | Moderate Wall (x{:.1})", ratio);
                        }
                    },
                    SignalType::Short => {
                         let ratio = if bid_wall > 0.0 { ask_wall / bid_wall } else { 0.0 };
                         if ratio > 1.2 {
                            signal.reason += &format!(" | Strong Sell Wall (x{:.1})", ratio);
                        } else {
                            signal.reason += &format!(" | Moderate Wall (x{:.1})", ratio);
                        }
                    }
                }
            }
        },
        Err(e) => warn!("Failed to fetch depth: {:?}", e),
    }

    // 2. Check Open Interest
    let oi_url = format!("https://fapi.binance.com/fapi/v1/openInterest?symbol={}", signal.symbol);
    match client.get(&oi_url).send().await {
        Ok(resp) => {
            if let Ok(oi_data) = resp.json::<OpenInterest>().await {
                if let Ok(oi_val) = oi_data.openInterest.parse::<f64>() {
                    let oi_in_usdt = oi_val * signal.price;
                     signal.reason += &format!(" | OI: ${:.1}M", oi_in_usdt / 1_000_000.0);
                     info!("Open Interest for {}: ${:.2}M", signal.symbol, oi_in_usdt / 1_000_000.0);
                }
            }
        },
        Err(e) => warn!("Failed to fetch OI: {:?}", e),
    }
    
    // 3. Net Inflow (Mock/Placeholder for now)
    // Real implementation would check Exchange Inflow API.
    // We add a "Whale Alert" tag if conditions meet.
    if signal.volume * signal.price > 5_000_000.0 {
         signal.reason += " | 🐋 Whale Active";
    }

    true 
}

fn calculate_wall(orders: Vec<[String; 2]>) -> f64 {
    let mut sum = 0.0;
    for order in orders {
        let qty: f64 = order[1].parse().unwrap_or(0.0);
        sum += qty;
    }
    sum
}
