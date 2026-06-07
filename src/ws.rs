use std::time::{SystemTime, UNIX_EPOCH};

use axum::{extract::{State, WebSocketUpgrade, ws::WebSocket}, response::IntoResponse};
use axum::extract::ws::Message as WsMessage;
use futures::StreamExt;
use futures::sink::SinkExt;
use time::Duration;
use tokio::sync::mpsc;
use tower_sessions::{Expiry, Session};

use crate::{message_handler::gen_outgoing_msg, structs::{AppData, ClientMessage, Message, User, UsernameResponse}};

pub async fn ws_upg(ws: WebSocketUpgrade, State(state): State<AppData>, session: Session) -> impl IntoResponse {
    ws.on_failed_upgrade(|error| println!("Error upgrading websocket: {}", error)).on_upgrade(move |socket| handle_socket(socket, state, session))
}

async fn handle_socket(socket: WebSocket, state: AppData, session: Session) {
    let mut user = session
        .get::<User>("userdata")
        .await
        .unwrap()
        .unwrap_or(User {
            id: "unknown".into(),
            name: "Unknown".into(),
            authenticated: false,
        });

    let (mut sink, mut stream) = socket.split();
    let mut rx = state.tx.subscribe();
    let (local_tx, mut local_rx) = mpsc::channel::<WsMessage>(32);

    
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(msg) = rx.recv() => {
                    let ws_msg = gen_outgoing_msg(&msg);
                    if sink.send(ws_msg).await.is_err() { break; }
                }
                Some(ws_msg) = local_rx.recv() => {
                    if sink.send(ws_msg).await.is_err() { break; }
                }
            }
        }
    });

    let username_information = UsernameResponse { username: user.name.clone(), authenticated: user.authenticated };
    if let Ok(json) = serde_json::to_string(&username_information) {
        let _ = local_tx.send(WsMessage::Text(json.into())).await;
    }   

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards, more than 50 years??").as_millis();
    let _ = state.tx.send(Message {
        owner: "SERVER_MESSAGE".to_string(),
        message: format!("User '{}' joined the chat!", user.name),
        date: timestamp as i64,
    });
    let _ = state.conn.lock().unwrap().execute("INSERT INTO messages (owner, date, content) VALUES (?1, ?2, ?3)", ("SERVER_MESSAGE", timestamp as i64, format!("User '{}' joined the chat!", user.name)));

    let mut messages = {
        let conn = state.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT owner, date, content
            FROM messages
            ORDER BY date DESC
            LIMIT 200"
        ).unwrap();

        let rows = stmt.query_map([], |row| {
            Ok(Message {
                owner: row.get(0)?,
                date: row.get(1)?,
                message: row.get(2)?,
            })
        }).unwrap();

        rows.map(|r| r.unwrap()).collect::<Vec<_>>()
    };
    messages.reverse();
    for message in messages {
        if let Ok(json) = serde_json::to_string(&message) {
            let _ = local_tx.send(WsMessage::Text(json.into())).await;
        } 
    }

    while let Some(Ok(msg)) = stream.next().await {
        if let WsMessage::Text(text) = msg {
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::Message(incoming)) => {
                    let _ = state.tx.send(Message {
                        owner: user.name.clone(),
                        message: incoming.content.clone(),
                        date: incoming.date,
                    });

                    let _ = state.conn.lock().unwrap().execute("INSERT INTO messages (owner, date, content) VALUES (?1, ?2, ?3)", (user.name.clone(), incoming.date, incoming.content));
                }

                Ok(ClientMessage::UsernameRequest(request)) => {
                    if user.authenticated == true {
                        if request.username == "SERVER_MESSAGE" { continue; }
                        let old_name = user.name.clone();
                        if old_name == request.username {continue;}
                        user.name = request.username;
                        session.set_expiry(Some(Expiry::AtDateTime(
                            time::OffsetDateTime::now_utc() + Duration::days(30)
                        )));
                        session.insert("userdata", &user).await.unwrap();
                        session.save().await.unwrap();
                        
                        let response = UsernameResponse { username: user.name.clone(), authenticated: user.authenticated };
                        if let Ok(json) = serde_json::to_string(&response) {
                            let _ = local_tx.send(WsMessage::Text(json.into())).await;
                        }

                        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards, more than 50 years??").as_millis();
                        let message = format!("User '{}' changed their username to '{}'", old_name, user.name);
                        let _ = state.tx.send(Message {
                            owner: "SERVER_MESSAGE".to_string(),
                            message: message.clone(),
                            date: timestamp as i64
                        });
                        let _ = state.conn.lock().unwrap().execute("INSERT INTO messages (owner, date, content) VALUES (?1, ?2, ?3)", ("SERVER_MESSAGE".to_string(), timestamp as i64, message));
                        let _ = state.conn.lock().unwrap().execute("UPDATE users SET username = ?1 WHERE id = ?2", (user.name.clone(), user.id.clone()));
                    } else {
                        let response = UsernameResponse { username: user.name.clone(), authenticated: user.authenticated };
                        if let Ok(json) = serde_json::to_string(&response) {
                            let _ = local_tx.send(WsMessage::Text(json.into())).await;
                        }
                    }
                }

                Err(err) => {
                    println!("Invalid message: {err}");
                }
            }
        }
    }

    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards, more than 50 years??").as_millis();
    let _ = state.tx.send(Message {
        owner: "SERVER_MESSAGE".to_string(),
        message: format!("User '{}' left the chat.", user.name),
        date: timestamp as i64,
    });
    let _ = state.conn.lock().unwrap().execute("INSERT INTO messages (owner, date, content) VALUES (?1, ?2, ?3)", ("SERVER_MESSAGE", timestamp as i64, format!("User '{}' left the chat.", user.name)));
}
