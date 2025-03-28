use bytes::Bytes;
use chrono::Utc;
use uuid::Uuid;

use super::config::Peer;
use super::message::{Message as AppMessage, MessageStatus};

pub mod dtchat {
    include!(concat!(env!("OUT_DIR"), "/dtchat.rs"));
}

pub fn serialize_message(message: &AppMessage) -> Bytes {
    #[cfg(debug_assertions)]
    return Bytes::from(format!("{}\n", message.text));

    #[cfg(not(debug_assertions))]
    {
        let proto_msg = construct_proto_message(message);
        let mut buf = bytes::BytesMut::with_capacity(proto_msg.encoded_len());
        use prost::Message;
        proto_msg.encode(&mut buf).unwrap();
        buf.freeze()
    }
}

pub fn deserialize_message(buf: &[u8], peers: &[Peer]) -> Option<AppMessage> {
    #[cfg(not(debug_assertions))]
    {
        use prost::Message;
        if let Ok(proto_msg) = dtchat::ChatMessage::decode(buf) {
            return extract_message_from_proto(proto_msg, peers);
        }
    }
    
    // Fallback to plain text for both debug mode and when protobuf parsing fails
    parse_text_message(buf, peers)
}

fn parse_text_message(buf: &[u8], peers: &[Peer]) -> Option<AppMessage> {
    if let Ok(text) = std::str::from_utf8(buf) {
        let text = text.trim();
        if !text.is_empty() {
            return Some(AppMessage {
                uuid: generate_uuid(),
                response: None,
                sender: peers.first()?.clone(),
                text: text.to_string(),
                shipment_status: MessageStatus::Received(Utc::now(), Utc::now()),
            });
        }
    }
    None
}

pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(not(debug_assertions))]
fn construct_proto_message(message: &AppMessage) -> dtchat::ChatMessage {
    use chrono::TimeZone;
    
    let (tx_time, _) = message.get_timestamps();
    
    let content = {
        let text_message = dtchat::TextMessage {
            content: message.text.clone(),
            reply_to_uuid: message.response.clone(),
        };
        Some(dtchat::chat_message::Content::Text(text_message))
    };

    dtchat::ChatMessage {
        uuid: message.uuid.clone(),
        sender_uuid: message.sender.uuid.clone(),
        timestamp: tx_time as i64,
        room_uuid: "default".to_string(), 
        content,
    }
}

#[cfg(not(debug_assertions))]
fn extract_message_from_proto(proto: dtchat::ChatMessage, peers: &[Peer]) -> Option<AppMessage> {
    use chrono::TimeZone;
    
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