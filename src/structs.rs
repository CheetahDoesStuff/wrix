use std::sync::{Arc, Mutex};

use rusqlite::Connection;
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UsernameResponse {
    pub username: String,
    pub authenticated: bool
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UsernameRequest {
    pub username: String
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub authenticated: bool
}

#[derive(Clone)]
pub struct AppData {
    pub tx: broadcast::Sender<Message>,
    pub conn: Arc<Mutex<Connection>>
}

#[derive(Deserialize)]
pub struct HcCallbackParams {
    pub code: String
}

#[derive(Serialize)]
pub struct HcTokenRequest<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub redirect_uri: &'a str,
    pub code: &'a str,
    pub grant_type: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct HcTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub id_token: Option<String>,
}


#[derive(Debug, Deserialize)]
pub struct HcClaims {
    pub sub: String,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub slack_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ClientMessage {
    Message(ReceivedData),
    UsernameRequest(UsernameRequest),
}