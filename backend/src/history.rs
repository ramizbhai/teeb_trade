use crate::scanner::Signal;
use crate::store::SharedState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalOutcome {
    pub price_at_15m: Option<f64>,
    pub price_at_30m: Option<f64>,
    pub price_at_60m: Option<f64>,
    pub success: bool,
    pub max_gain_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRecord {
    pub signal: Signal,
    pub outcome: SignalOutcome,
    pub recorded_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total_signals: usize,
    pub win_rate: f64,
    pub top_gainer: String, // e.g. "LINK +4.5%"
}

pub struct HistoryManager {
    records: Arc<Mutex<Vec<SignalRecord>>>,
    file_path: String,
}

impl HistoryManager {
    pub fn new(file_path: &str) -> Self {
        let records = if let Ok(data) = fs::read_to_string(file_path) {
            serde_json::from_str(&data).unwrap_or_else(|_| Vec::new())
        } else {
            Vec::new()
        };

        Self {
            records: Arc::new(Mutex::new(records)),
            file_path: file_path.to_string(),
        }
    }

    pub fn add_signal(&self, signal: Signal) {
        let mut records = self.records.lock().unwrap();
        records.push(SignalRecord {
            signal,
            outcome: SignalOutcome {
                price_at_15m: None,
                price_at_30m: None,
                price_at_60m: None,
                success: false,
                max_gain_percent: 0.0,
            },
            recorded_at: chrono::Utc::now().timestamp(),
        });
        self.save(&records);
    }

    fn save(&self, records: &Vec<SignalRecord>) {
        if let Ok(json) = serde_json::to_string(records) {
            let _ = fs::write(&self.file_path, json);
        }
    }

    pub fn get_stats(&self) -> Stats {
        let records = self.records.lock().unwrap();
        let total = records.len();
        if total == 0 {
            return Stats { total_signals: 0, win_rate: 0.0, top_gainer: "None".to_string() };
        }

        let wins = records.iter().filter(|r| r.outcome.success).count();
        let win_rate = (wins as f64 / total as f64) * 100.0;

        let best = records.iter()
            .max_by(|a, b| a.outcome.max_gain_percent.partial_cmp(&b.outcome.max_gain_percent).unwrap_or(std::cmp::Ordering::Equal));
        
        let top_gainer = match best {
            Some(r) => format!("{} {:.1}%", r.signal.symbol, r.outcome.max_gain_percent * 100.0),
            None => "None".to_string(),
        };

        Stats {
            total_signals: total,
            win_rate,
            top_gainer,
        }
    }

    pub fn get_recent_signals(&self) -> Vec<Signal> {
        let records = self.records.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        // Return signals from last 60 mins
        records.iter()
            .filter(|r| now - r.signal.timestamp < 60 * 60 * 1000)
            .map(|r| r.signal.clone())
            .collect()
    }

    pub fn update_outcomes(&self, store: SharedState) {
        let mut records = self.records.lock().unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        let mut updated = false;

        for record in records.iter_mut() {
            // Check milestones
            let elapsed_mins = (now - record.signal.timestamp) / 60000;
            
            // We need current price from store
            if let Some(state) = store.get(&record.signal.symbol) {
                 if let Some(last_data) = state.window.back() {
                     let current_price = last_data.price;
                     let entry_price = record.signal.price;
                     
                     // Calculate Gain for stats
                     let gain = match record.signal.signal_type {
                         crate::scanner::SignalType::Long => (current_price - entry_price) / entry_price,
                         crate::scanner::SignalType::Short => (entry_price - current_price) / entry_price,
                     };
                     
                     if gain > record.outcome.max_gain_percent {
                         record.outcome.max_gain_percent = gain;
                         updated = true;
                     }
                     
                     // Mark Success if gain > 1%
                     if gain > 0.01 {
                         record.outcome.success = true;
                         updated = true;
                     }

                     if elapsed_mins >= 15 && record.outcome.price_at_15m.is_none() {
                         record.outcome.price_at_15m = Some(current_price);
                         updated = true;
                     }
                     if elapsed_mins >= 30 && record.outcome.price_at_30m.is_none() {
                         record.outcome.price_at_30m = Some(current_price);
                         updated = true;
                     }
                     if elapsed_mins >= 60 && record.outcome.price_at_60m.is_none() {
                         record.outcome.price_at_60m = Some(current_price);
                         updated = true;
                     }
                 }
            }
        }
        
        if updated {
            self.save(&records);
        }
    }
}

pub async fn track_history(manager: Arc<HistoryManager>, store: SharedState, mut rx: broadcast::Receiver<crate::scanner::WsMessage>) {
    // 1. Listen for new signals
    let manager_clone = manager.clone();
    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let crate::scanner::WsMessage::Signal(signal) = msg {
                manager_clone.add_signal(signal);
            }
        }
    });

    // 2. Periodic Outcome Check (every 1 min)
    loop {
        manager.update_outcomes(store.clone());
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
