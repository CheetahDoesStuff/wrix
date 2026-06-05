use axum::{extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}}, response::IntoResponse};
use futures::StreamExt;
use futures::sink::SinkExt;
use tokio::sync::mpsc;

use crate::{message_handler::gen_outgoing_msg, structs::AppData};

pub async fn ws_upg(ws: WebSocketUpgrade, State(state): State<AppData>) -> impl IntoResponse {
    ws.on_failed_upgrade(|error| println!("Error upgrading websocket: {}", error)).on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppData) {
    let (mut sink, mut stream) = socket.split();
    
    let (tx, mut rx) = mpsc::unbounded_channel::<crate::structs::Message>();

    tokio::spawn(async move {
        while let Some(custom_msg) = rx.recv().await {
            let ws_msg = gen_outgoing_msg(&custom_msg); 
            if sink.send(ws_msg).await.is_err() {
                break;
            }
        }
    });

    {
        let mut sockets = state.sockets.lock().unwrap();
        sockets.push(tx); 
    }

    while let Some(Ok(msg)) = stream.next().await {
        if let Message::Text(text) = msg {
            if let Ok(incoming_custom_msg) = serde_json::from_str::<crate::structs::Message>(&text) {
                
                let sockets = state.sockets.lock().unwrap();
                for sender in sockets.iter() {
                    let _ = sender.send(incoming_custom_msg.clone());
                }
                
            } else {
                println!("Failed to parse incoming JSON payload");
            }
        }
    }
}