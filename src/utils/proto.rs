use bytes::{Bytes, BytesMut};
use chrono::{TimeZone, Utc};
use prost::Message as ProstMessage;
use uuid::Uuid;

use super::config::Peer;
use super::message::{Message as AppMessage, MessageStatus};

pub mod dtchat {
    include!(concat!(env!("OUT_DIR"), "/dtchat.rs"));
}

pub fn message_to_proto(message: &AppMessage) -> dtchat::ChatMessage {
    let (tx_time, _) = message.get_timestamps();
    
    let content = match &message.text {
        text => {
            let text_message = dtchat::TextMessage {
                content: text.to_string(),
                reply_to_uuid: message.response.clone(),
            };
            Some(dtchat::chat_message::Content::Text(text_message))
        }
    };

    dtchat::ChatMessage {
        uuid: message.uuid.clone(),
        sender_uuid: message.sender.uuid.clone(),
        timestamp: tx_time as i64,
        room_uuid: "default".to_string(), 
        content,
    }
}

pub fn proto_to_message(proto: dtchat::ChatMessage, peers: &[Peer]) -> Option<AppMessage> {
    let sender = peers.iter().find(|p| p.uuid == proto.sender_uuid)?;
    
    let content = proto.content.clone()?;
    
    let text = match &content {
        dtchat::chat_message::Content::Text(text_msg) => text_msg.content.clone(),
        _ => return None, 
    };
    
    let reply_to = match &content {
        dtchat::chat_message::Content::Text(text_msg) => text_msg.reply_to_uuid.clone(),
        _ => None,
    };
    
    let tx_time = Utc.timestamp_millis_opt(proto.timestamp).single()?;
    let rx_time = Utc::now();
    
    Some(AppMessage {
        uuid: proto.uuid,
        response: reply_to,
        sender: sender.clone(),
        text,
        shipment_status: MessageStatus::Received(tx_time, rx_time),
    })
}

pub fn serialize_message(message: &AppMessage) -> Bytes {
    let proto_msg = message_to_proto(message);
    let mut buf = BytesMut::with_capacity(proto_msg.encoded_len());
    proto_msg.encode(&mut buf).unwrap();
    buf.freeze()
}

pub fn deserialize_message(buf: &[u8], peers: &[Peer]) -> Option<AppMessage> {
    match dtchat::ChatMessage::decode(buf) {
        Ok(proto_msg) => proto_to_message(proto_msg, peers),
        Err(_) => None,
    }
}

pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
} 