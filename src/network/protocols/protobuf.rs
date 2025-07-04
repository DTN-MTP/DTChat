use crate::domain::{ChatMessage, MessageStatus, Peer};
use bytes::Bytes;
use chrono::{DateTime, Utc};

pub mod dtchat_proto {
    include!(concat!(env!("OUT_DIR"), "/dtchat.rs"));
}

pub use dtchat_proto::proto_message::Content;

#[derive(Debug)]
pub enum DeserializedMessage {
    ChatMessage(ChatMessage),
    Ack {
        message_uuid: String,
        is_read: bool,
        ack_time: DateTime<Utc>,
    },
}

pub fn serialize_message(message: &ChatMessage) -> Bytes {
    let proto_msg = construct_proto_message(message);
    let mut buf = bytes::BytesMut::with_capacity(proto_msg.encoded_len());
    use prost::Message;
    proto_msg.encode(&mut buf).unwrap();
    buf.freeze()
}

pub fn deserialize_message(buf: &[u8], peers: &[Peer]) -> Option<DeserializedMessage> {
    use prost::Message;
    match dtchat_proto::ProtoMessage::decode(buf) {
        Ok(proto_msg) => {
            extract_message_from_proto(proto_msg, peers)
        }
        Err(e) => {
            println!("❌ Failed to decode protobuf message: {e:?}");
            println!("❌ Buffer content: {buf:?}");
            None
        }
    }
}

fn find_peer_by_id(peers: &[Peer], id: &str) -> Option<Peer> {
    peers.iter().find(|p| p.uuid == id).cloned()
}

fn default_peer() -> Peer {
    Peer::default()
}

fn construct_proto_message(message: &ChatMessage) -> dtchat_proto::ProtoMessage {
    let (tx_time, _, _) = message.get_timestamps();

    let content = {
        let text_message = dtchat_proto::TextMessage {
            content: message.text.clone(),
            reply_to_uuid: message.response.clone(),
        };
        Some(Content::Text(text_message))
    };

    dtchat_proto::ProtoMessage {
        uuid: message.uuid.clone(),
        sender_uuid: message.sender.uuid.clone(),
        timestamp: tx_time as i64,
        room_uuid: "default".to_string(),
        content,
    }
}

fn extract_message_from_proto(
    proto: dtchat_proto::ProtoMessage,
    peers: &[Peer],
) -> Option<DeserializedMessage> {
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
