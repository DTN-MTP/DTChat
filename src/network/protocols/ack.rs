use crate::domain::ChatMessage;
use crate::network::protocols::protobuf::dtchat_proto;
use crate::network::protocols::protobuf::dtchat_proto::proto_message::Content;
use crate::network::protocols::protobuf::dtchat_proto::DeliveryStatus;
use crate::network::socket::{GenericSocket, TOKIO_RUNTIME};
use crate::utils::generate_uuid;

pub type AckResult<T> = Result<T, AckError>;

#[derive(Debug)]
pub enum AckError {
    Network(Box<dyn std::error::Error + Send + Sync>),
    #[allow(dead_code)]
    Serialization(String),
    #[allow(dead_code)]
    InvalidMessage(String), // Invalid message format
}

impl std::fmt::Display for AckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(err) => write!(f, "Network error during ACK: {err}"),
            Self::Serialization(msg) => write!(f, "Serialization error during ACK: {msg}"),
            Self::InvalidMessage(msg) => write!(f, "Invalid message format for ACK: {msg}"),
        }
    }
}

impl std::error::Error for AckError {}

#[allow(dead_code)]
pub fn create_ack_message(
    received_msg: &ChatMessage,
    local_peer_uuid: &str,
    is_read: bool,
) -> dtchat_proto::ProtoMessage {
    let delivery_status = DeliveryStatus {
        message_uuid: received_msg.uuid.clone(),
        received: true,
        read: is_read,
    };

    let ack_uuid = generate_uuid();

    dtchat_proto::ProtoMessage {
        uuid: ack_uuid,
        sender_uuid: local_peer_uuid.to_string(), // ACK is sent by the local peer
        timestamp: chrono::Utc::now().timestamp_millis(),
        room_uuid: "default".to_string(), // Using default room
        content: Some(Content::Delivery(delivery_status)),
    }
}

pub async fn send_ack_message(
    received_msg: &ChatMessage,
    socket: &mut GenericSocket,
    local_peer_uuid: &str,
    is_read: bool,
) -> AckResult<()> {
    use prost::Message;
    let ack_proto_msg = create_ack_message(received_msg, local_peer_uuid, is_read);

    let mut buf = bytes::BytesMut::with_capacity(ack_proto_msg.encoded_len());

    if let Err(e) = prost::Message::encode(&ack_proto_msg, &mut buf) {
        println!(
            "❌ ACK encoding failed for message {}: {}",
            received_msg.uuid, e
        );
        return Err(AckError::Serialization(e.to_string()));
    }

    let buf_len = buf.len();
    match socket.send(&buf.freeze()) {
        Ok(_) => {
            println!(
                "✅ ACK sent successfully for message {} (read: {}, size: {} bytes)",
                received_msg.uuid, is_read, buf_len
            );
            Ok(())
        }
        Err(e) => {
            println!(
                "❌ ACK transmission failed for message {}: {}",
                received_msg.uuid, e
            );
            Err(AckError::Network(Box::new(e)))
        }
    }
}

pub fn send_ack_message_non_blocking(
    received_msg: &ChatMessage,
    socket: &mut GenericSocket,
    local_peer_uuid: &str,
    is_read: bool,
) {

    let msg_clone = received_msg.clone();
    let mut socket_clone = socket.clone();
    let local_peer_uuid_clone = local_peer_uuid.to_string();

    TOKIO_RUNTIME.spawn(async move {
        if let Err(e) = send_ack_message(
            &msg_clone,
            &mut socket_clone,
            &local_peer_uuid_clone,
            is_read,
        )
        .await
        {
            eprintln!("❌ ACK task failed for message {}: {}", msg_clone.uuid, e);
        } 
    });
}
