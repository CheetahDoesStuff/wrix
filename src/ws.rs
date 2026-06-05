use axum::{extract::{State, WebSocketUpgrade, ws::WebSocket}, response::IntoResponse};
use axum::extract::ws::Message as WsMessage;
use futures::StreamExt;
use futures::sink::SinkExt;
use tower_sessions::Session;

use crate::{message_handler::gen_outgoing_msg, structs::{AppData, Message, User}};

pub async fn ws_upg(ws: WebSocketUpgrade, State(state): State<AppData>, session: Session) -> impl IntoResponse {
    ws.on_failed_upgrade(|error| println!("Error upgrading websocket: {}", error)).on_upgrade(move |socket| handle_socket(socket, state, session))
}

async fn handle_socket(socket: WebSocket, state: AppData, session: Session) {
    let user = session
        .get::<User>("user")
        .await
        .unwrap()
        .unwrap_or(User {
            id: "unknown".into(),
            name: "Unknown".into(),
            authenticated: false,
        });

    let (mut sink, mut stream) = socket.split();
    let mut rx = state.tx.subscribe();

    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let ws_msg = gen_outgoing_msg(&msg);

            if sink.send(ws_msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        if let WsMessage::Text(text) = msg {
            if let Ok(incoming) =
                serde_json::from_str::<crate::structs::ReceivedData>(&text)
            {
                let _ = state.tx.send(
                    Message {
                        owner: user.name.clone(),
                        message: incoming.content.clone(),
                        date: incoming.date,
                    }
                );
            }
        } else {
            println!("Failed to parse incoming JSON payload");        
        }
    }
}