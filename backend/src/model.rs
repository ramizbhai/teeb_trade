use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct SymbolState {
    pub symbol: String,
    // Sliding window of the last 60 minutes
    pub window: VecDeque<MarketData>,
    pub last_signal_time: Option<i64>,
}

impl SymbolState {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            window: VecDeque::new(),
            last_signal_time: None,
        }
    }

    pub fn add_data(&mut self, data: MarketData) {
        if self.window.len() >= 60 {
            self.window.pop_front();
        }
        self.window.push_back(data);
    }
    
    pub fn get_average_volume(&self) -> f64 {
        if self.window.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.window.iter().map(|d| d.volume).sum();
        sum / self.window.len() as f64
    }
}
