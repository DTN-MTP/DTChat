use bytes::Bytes;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::config::Peer;
use super::message::{ChatMessage, MessageStatus};

pub mod dtchat {
    include!(concat!(env!("OUT_DIR"), "/dtchat.rs"));
}

pub use dtchat::chat_message::Content;

#[derive(Debug)]
pub enum DeserializedMessage {
    ChatMessage(ChatMessage),
    Ack { message_uuid: String, is_read: bool, ack_time: DateTime<Utc> },
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

pub fn deserialize_message(buf: &[u8], peers: &[Peer]) -> Option<DeserializedMessage> {
    #[cfg(not(debug_assertions))]
    {
        use prost::Message;
        if let Ok(proto_msg) = dtchat::ChatMessage::decode(buf) {
            return extract_message_from_proto(proto_msg, peers);
        }
    }

    // Handle text-based messages (debug mode)
    if let Ok(text) = std::str::from_utf8(buf) {
        let text = text.trim_end();
        if !text.is_empty() {
            let now = Utc::now();
            
            // Check if this is an ACK message in debug mode
            if text.starts_with("[ACK]") {
                // Parse ACK format: [ACK] message_uuid:is_read
                if let Some(ack_data) = text.strip_prefix("[ACK] ") {
                    let parts: Vec<&str> = ack_data.split(':').collect();
                    if parts.len() == 2 {
                        let message_uuid = parts[0].to_string();
                        let is_read = parts[1] == "true";
                        return Some(DeserializedMessage::Ack {
                            message_uuid,
                            is_read,
                            ack_time: now,
                        });
                    }
                }
            }

            let default_peer = find_peer_by_id(peers, "0").unwrap_or_else(default_peer);

            return Some(DeserializedMessage::ChatMessage(ChatMessage {
                uuid: generate_uuid(),
                response: None,
                sender: default_peer,
                text: text.to_string(),
                shipment_status: MessageStatus::Received(now, now),
            }));
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
        Some(Content::Text(text_message))
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
fn extract_message_from_proto(proto: dtchat::ChatMessage, peers: &[Peer]) -> Option<DeserializedMessage> {
    use chrono::TimeZone;

    let sender = find_peer_by_id(peers, &proto.sender_uuid).unwrap_or_else(default_peer);

    let content = proto.content.clone()?;

    // Handle ACK messages separately
    if let Content::Delivery(delivery_status) = &content {
        let ack_time = Utc.timestamp_millis_opt(proto.timestamp).single()?;
        return Some(DeserializedMessage::Ack {
            message_uuid: delivery_status.message_uuid.clone(),
            is_read: delivery_status.read,
            ack_time,
        });
    }

    // Extract text based on the message type
    let (text, reply_to) = match &content {
        Content::Text(text_msg) => (text_msg.content.clone(), text_msg.reply_to_uuid.clone()),
        Content::File(_) => (
            "File transfer (not implemented for display)".to_string(),
            None,
        ),
        Content::Presence(_) => (
            "Presence update (not implemented for display)".to_string(),
            None,
        ),
        Content::Delivery(_) => unreachable!(), // Already handled above
    };

    let tx_time = Utc.timestamp_millis_opt(proto.timestamp).single()?;
    let rx_time = Utc::now();

    Some(DeserializedMessage::ChatMessage(ChatMessage {
        uuid: proto.uuid,
        response: reply_to,
        sender,
        text,
        shipment_status: MessageStatus::Received(tx_time, rx_time),
    }))
}

/// Serialize an ACK message for debug mode
pub fn serialize_ack_debug(message_uuid: &str, is_read: bool) -> Bytes {
    let ack_content = format!("[ACK] {}:{}\n", message_uuid, is_read);
    Bytes::from(ack_content)
}
