use std::env;
use tokio::time::{sleep, Duration};

use crate::utils::message::ChatMessage;
use crate::utils::proto::dtchat::chat_message::Content;
use crate::utils::proto::dtchat::DeliveryStatus;
use crate::utils::proto::{dtchat, generate_uuid};
use crate::utils::socket::{self, GenericSocket};

pub struct AckConfig {
    pub delay_enabled: bool,
    pub delay_duration_ms: u64,
}

impl Default for AckConfig {
    fn default() -> Self {
        // Read delay duration from environment variable or use default
        let delay_ms = env::var("DTCHAT_ACK_DELAY_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(500); // Default to 500ms

        Self {
            delay_enabled: cfg!(feature = "delayed_ack"),
            delay_duration_ms: delay_ms,
        }
    }
}

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
            Self::Network(err) => write!(f, "Network error during ACK: {}", err),
            Self::Serialization(msg) => write!(f, "Serialization error during ACK: {}", msg),
            Self::InvalidMessage(msg) => write!(f, "Invalid message format for ACK: {}", msg),
        }
    }
}

impl std::error::Error for AckError {}

#[allow(dead_code)]
pub fn create_ack_message(received_msg: &ChatMessage, local_peer_uuid: &str, is_read: bool) -> dtchat::ChatMessage {
    let delivery_status = DeliveryStatus {
        message_uuid: received_msg.uuid.clone(),
        received: true,
        read: is_read,
    };

    dtchat::ChatMessage {
        uuid: generate_uuid(),
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
    config: Option<AckConfig>,
) -> AckResult<()> {
    let config = config.unwrap_or_default();

    if config.delay_enabled {
        sleep(Duration::from_millis(config.delay_duration_ms)).await;
    }

    // Send ACK based on build mode
    #[cfg(feature = "no_protobuf")]
    {
        use crate::utils::proto::serialize_ack_debug;
        let ack_data = serialize_ack_debug(&received_msg.uuid, is_read);
        match socket.send(&ack_data) {
            Ok(_) => {
                println!("Sent ACK for message {} (read: {})", received_msg.uuid, is_read);
                Ok(())
            },
            Err(e) => Err(AckError::Network(e)),
        }
    }

    #[cfg(not(feature = "no_protobuf"))]
    {
        use prost::Message;

        let ack_proto_msg = create_ack_message(received_msg, local_peer_uuid, is_read);

        let mut buf = bytes::BytesMut::with_capacity(ack_proto_msg.encoded_len());

        if let Err(e) = prost::Message::encode(&ack_proto_msg, &mut buf) {
            return Err(AckError::Serialization(e.to_string()));
        }

        match socket.send(&buf.freeze()) {
            Ok(_) => {
                println!("Sent protobuf ACK for message {} (read: {})", received_msg.uuid, is_read);
                Ok(())
            },
            Err(e) => Err(AckError::Network(e)),
        }
    }
}

pub fn send_ack_message_non_blocking(
    received_msg: &ChatMessage,
    socket: &mut GenericSocket,
    local_peer_uuid: &str,
    is_read: bool,
    config: Option<AckConfig>,
) {
    let msg_clone = received_msg.clone();
    let mut socket_clone = socket.clone();
    let local_peer_uuid_clone = local_peer_uuid.to_string();

    socket::TOKIO_RUNTIME.spawn(async move {
        if let Err(e) = send_ack_message(&msg_clone, &mut socket_clone, &local_peer_uuid_clone, is_read, config).await {
            eprintln!("Failed to send ACK message: {}", e);
        }
    });
}
