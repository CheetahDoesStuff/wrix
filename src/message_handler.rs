use crate::structs::Message;
use axum::extract::ws::Message as WsMessage;

pub fn gen_outgoing_msg(msg: &Message) -> WsMessage {
    let json_string = serde_json::to_string(msg).unwrap();
    WsMessage::text(json_string)
}

pub fn process_incoming_msg(raw_ws: &str) -> Result<Message, serde_json::Error> {
    serde_json::from_str::<Message>(raw_ws)
}