use crate::domain::{ChatMessage, Peer};
use crate::network::protocols::{deserialize_message, serialize_message, DeserializedMessage};
use crate::network::NetworkResult;
use bytes::Bytes;

pub trait MessageSerializer {
    fn serialize(&self, message: &ChatMessage) -> NetworkResult<Bytes>;
    fn deserialize(
        &self,
        data: &[u8],
        peers: &[Peer],
    ) -> NetworkResult<Option<DeserializedMessage>>;
}

pub struct ProtobufSerializer;

impl MessageSerializer for ProtobufSerializer {
    fn serialize(&self, message: &ChatMessage) -> NetworkResult<Bytes> {
        Ok(serialize_message(message))
    }

    fn deserialize(
        &self,
        data: &[u8],
        peers: &[Peer],
    ) -> NetworkResult<Option<DeserializedMessage>> {
        Ok(deserialize_message(data, peers))
    }
}

pub struct MessageSerializerEngine {
    serializer: Box<dyn MessageSerializer + Send + Sync>,
}

impl MessageSerializerEngine {
    pub fn new() -> Self {
        Self {
            serializer: Box::new(ProtobufSerializer),
        }
    }

    pub fn encode(&self, message: &ChatMessage) -> NetworkResult<Vec<u8>> {
        let bytes = self.serializer.serialize(message)?;
        Ok(bytes.to_vec())
    }

    pub fn decode(
        &self,
        data: &[u8],
        peers: &[Peer],
    ) -> NetworkResult<Option<DeserializedMessage>> {
        if data.is_empty() {
            return Ok(None);
        }

        self.serializer.deserialize(data, peers)
    }
}

impl Default for MessageSerializerEngine {
    fn default() -> Self {
        Self::new()
    }
}
