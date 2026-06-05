use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub owner: String,
    pub message: String,
    pub date: i64
}

#[derive(Clone)]
pub struct AppData {
    pub sockets: Arc<Mutex<Vec<mpsc::UnboundedSender<Message>>>>
}