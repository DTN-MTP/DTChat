use crate::domain::ChatMessage;
use crate::network::{
    encoding::MessageSerializerEngine, peer_manager::PeerManager, protocols::ack,
    socket::GenericSocket, Endpoint, NetworkError, NetworkResult,
};
use std::sync::{Arc, Mutex};

pub struct MessageRouter {
    peer_manager: Arc<Mutex<PeerManager>>,
}

impl MessageRouter {
    pub fn new(peer_manager: Arc<Mutex<PeerManager>>) -> Self {
        Self { peer_manager }
    }

    pub fn send_message_to_peer(
        &self,
        message: &ChatMessage,
        peer_uuid: &str,
    ) -> NetworkResult<()> {
        let peer_manager = self.peer_manager.lock().unwrap();
        let target_peer = peer_manager
            .find_peer(peer_uuid)
            .ok_or_else(|| NetworkError::InvalidFormat(format!("Peer not found: {peer_uuid}")))?;

        for endpoint in &target_peer.endpoints {
            if endpoint.is_valid() {
                match self.send_message_to_endpoint(message, endpoint) {
                    Ok(_) => {
                        println!("📤 Message sent to {} via {}", target_peer.name, endpoint);
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Failed to send via {endpoint}: {e}");
                        continue;
                    }
                }
            }
        }

        Err(NetworkError::InvalidFormat(format!(
            "No valid endpoints for peer: {peer_uuid}"
        )))
    }

    pub fn send_message_to_endpoint(
        &self,
        message: &ChatMessage,
        endpoint: &Endpoint,
    ) -> NetworkResult<()> {
        let mut socket = GenericSocket::new(endpoint.clone())?;
        let serializer = MessageSerializerEngine::new();
        let data = serializer.encode(message)?;
        socket.send(&data)?;
        Ok(())
    }

    pub fn send_ack(
        &self,
        original_message: &ChatMessage,
        target_endpoint: &Endpoint,
    ) -> NetworkResult<()> {
        let peer_manager = self.peer_manager.lock().unwrap();
        let local_peer_uuid = &peer_manager.local_peer().uuid;

        let mut socket = GenericSocket::new(target_endpoint.clone())?;
        ack::send_ack_message_non_blocking(
            original_message,
            &mut socket,
            local_peer_uuid,
            false, // Not read yet, just received
        );
        Ok(())
    }
}
