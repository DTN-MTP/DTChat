use crate::network::{NetworkError, NetworkResult};
use crate::utils::{config::Peer, message::ChatMessage, proto::{serialize_message, deserialize_message, DeserializedMessage}};
use bytes::Bytes;

/// Trait for message serialization
pub trait MessageSerializer {
    fn serialize(&self, message: &ChatMessage) -> NetworkResult<Bytes>;
    fn deserialize(&self, data: &[u8], peers: &[Peer]) -> NetworkResult<Option<DeserializedMessage>>;
}

/// Default protobuf-based message serializer
pub struct ProtobufSerializer;

impl MessageSerializer for ProtobufSerializer {
    fn serialize(&self, message: &ChatMessage) -> NetworkResult<Bytes> {
        Ok(serialize_message(message))
    }

    fn deserialize(&self, data: &[u8], peers: &[Peer]) -> NetworkResult<Option<DeserializedMessage>> {
        Ok(deserialize_message(data, peers))
    }
}

/// Message encoder/decoder with validation
pub struct MessageCodec {
    serializer: Box<dyn MessageSerializer + Send + Sync>,
}

impl MessageCodec {
    pub fn new() -> Self {
        Self {
            serializer: Box::new(ProtobufSerializer),
        }
    }

    pub fn with_serializer(serializer: Box<dyn MessageSerializer + Send + Sync>) -> Self {
        Self { serializer }
    }

    /// Encode a message to bytes
    pub fn encode(&self, message: &ChatMessage) -> NetworkResult<Vec<u8>> {
        let bytes = self.serializer.serialize(message)?;
        Ok(bytes.to_vec())
    }

    /// Decode bytes to a message
    pub fn decode(&self, data: &[u8], peers: &[Peer]) -> NetworkResult<Option<DeserializedMessage>> {
        if data.is_empty() {
            return Ok(None);
        }

        self.serializer.deserialize(data, peers)
    }

    /// Validate message before encoding
    pub fn validate_message(&self, message: &ChatMessage) -> NetworkResult<()> {
        if message.text.is_empty() {
            return Err(NetworkError::InvalidFormat("Message text cannot be empty".to_string()));
        }

        if message.sender.uuid.is_empty() {
            return Err(NetworkError::InvalidFormat("Sender UUID cannot be empty".to_string()));
        }

        // Note: room_uuid validation removed as it's not part of ChatMessage structure

        Ok(())
    }

    /// Encode with validation
    pub fn encode_validated(&self, message: &ChatMessage) -> NetworkResult<Vec<u8>> {
        self.validate_message(message)?;
        self.encode(message)
    }
}

impl Default for MessageCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// Message framing for network transmission
pub struct MessageFrame {
    pub size: u32,
    pub data: Vec<u8>,
}

impl MessageFrame {
    /// Create a new frame from data
    pub fn new(data: Vec<u8>) -> Self {
        let size = data.len() as u32;
        Self { size, data }
    }

    /// Serialize frame to bytes (size prefix + data)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(4 + self.data.len());
        result.extend_from_slice(&self.size.to_be_bytes());
        result.extend_from_slice(&self.data);
        result
    }

    /// Deserialize frame from bytes
    pub fn from_bytes(bytes: &[u8]) -> NetworkResult<Self> {
        if bytes.len() < 4 {
            return Err(NetworkError::InvalidFormat("Frame too short for size header".to_string()));
        }

        let size_bytes: [u8; 4] = bytes[0..4].try_into()
            .map_err(|_| NetworkError::InvalidFormat("Invalid size header".to_string()))?;
        let size = u32::from_be_bytes(size_bytes);

        if bytes.len() < 4 + size as usize {
            return Err(NetworkError::InvalidFormat("Frame shorter than declared size".to_string()));
        }

        let data = bytes[4..4 + size as usize].to_vec();
        Ok(Self { size, data })
    }

    /// Get the data payload
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the frame size
    pub fn size(&self) -> u32 {
        self.size
    }
}

// TODO: Add proper tests with correct ChatMessage structure