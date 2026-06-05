use std::sync::{Arc, Mutex};

use axum::{Router, routing::get};
use tower_http::services::{ServeDir, ServeFile};

use crate::structs::AppData;

pub mod ws;
pub mod structs;
pub mod message_handler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_state = AppData {
        sockets: Arc::new(Mutex::new(Vec::new()))
    };

    let api_router = Router::new()
        .route("/ws", get(ws::ws_upg))
        .with_state(app_state);

    let page_router = Router::new()
        .nest_service("/chat", ServeFile::new("public/chat.html"));

    let app = Router::new()
        .merge(api_router)
        .merge(page_router);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}