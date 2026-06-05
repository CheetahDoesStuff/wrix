use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub owner: String,
    pub message: String,
    pub date: i64
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReceivedData {
    pub content_type: String,
    pub content: String,
    pub date: i64
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub authenticated: bool
}

#[derive(Clone)]
pub struct AppData {
    pub tx: broadcast::Sender<Message>
}