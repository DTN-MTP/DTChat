use bytes::Bytes;
use chrono::Utc;
use uuid::Uuid;

use super::config::Peer;
use super::message::{ChatMessage, MessageStatus};

pub mod dtchat {
    include!(concat!(env!("OUT_DIR"), "/dtchat.rs"));
}

pub fn serialize_message(message: &ChatMessage) -> Bytes {
    #[cfg(debug_assertions)]
    return Bytes::from(message.text.clone() + "\n");

    #[cfg(not(debug_assertions))]
    {
        let proto_msg = construct_proto_message(message);
        let mut buf = bytes::BytesMut::with_capacity(proto_msg.encoded_len());
        use prost::Message;
        proto_msg.encode(&mut buf).unwrap();
        buf.freeze()
    }
}

pub fn deserialize_message(buf: &[u8], peers: &[Peer]) -> Option<ChatMessage> {
    #[cfg(not(debug_assertions))]
    {
        use prost::Message;
        if let Ok(proto_msg) = dtchat::ChatMessage::decode(buf) {
            return extract_message_from_proto(proto_msg, peers);
        }
    }

    if let Ok(text) = std::str::from_utf8(buf) {
        let text = text.trim_end();
        if !text.is_empty() {
            let now = Utc::now();
            let default_peer = find_peer_by_id(peers, "0").unwrap_or_else(default_peer);

            return Some(ChatMessage {
                uuid: generate_uuid(),
                response: None,
                sender: default_peer,
                text: text.to_string(),
                shipment_status: MessageStatus::Received(now, now),
                pbat_enabled : false,
            });
        }
    }
    None
}

fn find_peer_by_id(peers: &[Peer], id: &str) -> Option<Peer> {
    peers.iter().find(|p| p.uuid == id).cloned()
}

fn default_peer() -> Peer {
    Peer::default()
}

pub fn create_message(text: &str, sender: Peer) -> ChatMessage {
    ChatMessage {
        uuid: generate_uuid(),
        response: None,
        sender,
        text: text.to_string(),
        shipment_status: MessageStatus::Received(Utc::now(), Utc::now()),
        pbat_enabled : false,
    }
}

pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(not(debug_assertions))]
fn construct_proto_message(message: &ChatMessage) -> dtchat::ChatMessage {
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
fn extract_message_from_proto(proto: dtchat::ChatMessage, peers: &[Peer]) -> Option<ChatMessage> {
    use chrono::TimeZone;

    let sender = find_peer_by_id(peers, &proto.sender_uuid).unwrap_or_else(default_peer);

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

    Some(ChatMessage {
        uuid: proto.uuid,
        response: reply_to,
        sender,
        text,
        shipment_status: MessageStatus::Received(tx_time, rx_time),
    })
}
