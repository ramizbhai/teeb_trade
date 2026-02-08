mod model;
mod store;
mod scanner;
mod binance_client;
mod ws_server;
mod verifier;
mod history;

use tokio::sync::broadcast;
use log::info;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    info!("Starting Teeb Trade Backend (Rust)...");

    // Initialize Shared State
    let store = store::init_store();

    use scanner::WsMessage;
    // Initialize Signal Channel
    let (tx, _rx) = broadcast::channel::<WsMessage>(100);

    // Initialize History Manager
    let history_manager = std::sync::Arc::new(history::HistoryManager::new("history.json"));
    
    // Spawn History Tracker
    let history_store = store.clone();
    let history_tx = tx.clone();
    let history_manager_clone = history_manager.clone();
    tokio::spawn(async move {
        // subscribe to rx for history
        let rx = history_tx.subscribe();
        // We need to implement the async function properly in history.rs or call methods.
        // Wait, `track_history` takes `rx`.
        history::track_history(history_manager_clone, history_store, rx).await;
    });

    // Spawn Binance WebSocket Client
    let store_clone = store.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        binance_client::binance_ws_task(store_clone, tx_clone).await;
    });

    // Spawn Frontend WebSocket Server
    let history_manager_for_server = history_manager.clone();
    tokio::spawn(async move {
        ws_server::start_ws_server(tx, history_manager_for_server).await;
    });

    // Keep main thread alive
    tokio::signal::ctrl_c().await.unwrap();
    info!("Shutting down...");
}
