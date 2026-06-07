use std::sync::{Arc, Mutex};

use axum::middleware;
use axum::{Router, routing::get};
use rusqlite::Connection;
use tokio::sync::broadcast;
use tower_http::services::{ServeFile};
use tower_sessions::cookie::SameSite;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::auth::{logout, require_auth};
use crate::{auth::{hc_auth_redirect, hc_callback, login_guest, root_handler}, structs::{AppData, Message}};

pub mod ws;
pub mod structs;
pub mod message_handler;
pub mod auth;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, _) = broadcast::channel::<Message>(100);
    let app_state = AppData { tx: tx, conn: Arc::new(Mutex::new(Connection::open("data.db").unwrap())) };
    let _ = app_state.conn.lock().unwrap().execute("CREATE TABLE IF NOT EXISTS users ( id TEXT, username TEXT )", ());
    let _ = app_state.conn.lock().unwrap().execute("CREATE TABLE IF NOT EXISTS messages ( owner TEXT, date INTEGER, content TEXT )", ());

    let store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(store)
        .with_secure(false)
        .with_same_site(SameSite::Lax);

    let api_router = Router::new()
        .route("/auth/logout", get(logout))
        .route("/auth/guest", get(login_guest))
        .route("/auth/hc", get(hc_auth_redirect))
        .route("/auth/hc/callback", get(hc_callback))
        .route("/ws", get(ws::ws_upg))
        .route("/", get(root_handler))
        .with_state(app_state);

    let auth_router = Router::new()
        .nest_service("/chat", ServeFile::new("public/chat.html"))
        .route_layer(middleware::from_fn(require_auth));

    let page_router = Router::new()
        .merge(auth_router)
        .nest_service("/login", ServeFile::new("public/login.html"))
        .nest_service("/style.css", ServeFile::new("public/style.css"));

    let app = Router::new()
        .merge(api_router)
        .merge(page_router)
        .layer(session_layer);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;

    Ok(())
}