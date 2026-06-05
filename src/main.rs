use std::sync::{Arc, Mutex};

use axum::{Router, routing::get};

use crate::structs::AppData;

pub mod ws;
pub mod structs;
pub mod message_handler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_state = AppData {
        sockets: Arc::new(Mutex::new(Vec::new()))
    };

    let app = Router::new().route("/ws", get(ws::ws_upg)).with_state(app_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}