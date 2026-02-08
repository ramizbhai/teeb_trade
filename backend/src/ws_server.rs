use warp::Filter;
use tokio::sync::broadcast;
use futures_util::{StreamExt, SinkExt};
use log::{info, error};
use crate::scanner::WsMessage;
use crate::history::HistoryManager;
use std::sync::Arc;

pub async fn start_ws_server(tx: broadcast::Sender<WsMessage>, history: Arc<HistoryManager>) {
    let tx = warp::any().map(move || tx.clone());
    let history = warp::any().map(move || history.clone());

    let routes = warp::path("ws")
        .and(warp::ws())
        .and(tx)
        .and(history)
        .map(|ws: warp::ws::Ws, tx: broadcast::Sender<WsMessage>, history: Arc<HistoryManager>| {
            ws.on_upgrade(move |socket| handle_client(socket, tx, history))
        });

    info!("Starting WebSocket Signal Server on 0.0.0.0:3000");
    warp::serve(routes).run(([0, 0, 0, 0], 3000)).await;
}

async fn handle_client(ws: warp::ws::WebSocket, tx: broadcast::Sender<WsMessage>, history: Arc<HistoryManager>) {
    let (mut client_ws_tx, _) = ws.split();
    let mut rx = tx.subscribe();

    info!("New Frontend Client Connected");

    // Send Initial Stats
    let stats = history.get_stats();
    // Send as WsMessage::Stats
    if let Ok(json) = serde_json::to_string(&WsMessage::Stats(stats)) {
        let _ = client_ws_tx.send(warp::ws::Message::text(json)).await;
    }
    
    // Send History (Last 60 mins)
    let recent_signals = history.get_recent_signals();
    if !recent_signals.is_empty() {
        if let Ok(json) = serde_json::to_string(&WsMessage::History(recent_signals)) {
            let _ = client_ws_tx.send(warp::ws::Message::text(json)).await;
        }
    }

    while let Ok(msg) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&msg) {
            if let Err(e) = client_ws_tx.send(warp::ws::Message::text(json)).await {
                error!("Failed to send signal to client: {:?}", e);
                break;
            }
        }
    }
    info!("Client Disconnected");
}
