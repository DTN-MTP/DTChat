use crate::network::{
    socket::{DefaultSocketController, GenericSocket, SocketController, SocketObserver},
    Endpoint, NetworkError, NetworkResult,
};
use crate::utils::{
    ack,
    config::Peer,
    message::ChatMessage,
};
use std::sync::{Arc, Mutex};

/// Network engine for managing connections and message routing
pub struct NetworkEngine {
    controller: Arc<Mutex<DefaultSocketController>>,
    local_peer: Peer,
    peers: Vec<Peer>,
}

impl NetworkEngine {
    /// Create a new NetworkEngine instance
    pub fn new(local_peer: Peer, peers: Vec<Peer>) -> NetworkResult<Self> {
        let controller = DefaultSocketController::init_controller(local_peer.clone(), peers.clone())?;
        
        Ok(Self {
            controller,
            local_peer,
            peers,
        })
    }

    /// Add an observer to the network engine
    pub fn add_observer(&self, observer: Arc<dyn SocketObserver>) {
        let mut controller = self.controller.lock().unwrap();
        controller.add_observer(observer);
    }

    /// Send a message to a specific peer
    pub fn send_message_to_peer(&self, message: &ChatMessage, peer_uuid: &str) -> NetworkResult<()> {
        let target_peer = self.peers.iter()
            .find(|p| p.uuid == peer_uuid)
            .ok_or_else(|| NetworkError::InvalidFormat(format!("Peer not found: {}", peer_uuid)))?;

        // Try to send using the first valid endpoint
        for endpoint in &target_peer.endpoints {
            if endpoint.is_valid() {
                match self.send_message_to_endpoint(message, endpoint) {
                    Ok(_) => {
                        println!("📤 Message sent to {} via {}", target_peer.name, endpoint);
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("Failed to send via {}: {}", endpoint, e);
                        continue;
                    }
                }
            }
        }

        Err(NetworkError::InvalidFormat(format!("No valid endpoints for peer: {}", peer_uuid)))
    }

    /// Send a message to a specific endpoint
    pub fn send_message_to_endpoint(&self, message: &ChatMessage, endpoint: &Endpoint) -> NetworkResult<()> {
        let mut socket = GenericSocket::new(endpoint.clone())?;
        socket.send_message(message)?;
        Ok(())
    }

    /// Send an ACK message
    pub fn send_ack(&self, original_message: &ChatMessage, target_endpoint: &Endpoint) -> NetworkResult<()> {
        let mut socket = GenericSocket::new(target_endpoint.clone())?;
        ack::send_ack_message_non_blocking(
            original_message,
            &mut socket,
            &self.local_peer.uuid,
            false, // Not read yet, just received
        );
        Ok(())
    }

    /// Get the local peer
    pub fn local_peer(&self) -> &Peer {
        &self.local_peer
    }

    /// Get all peers
    pub fn peers(&self) -> &[Peer] {
        &self.peers
    }

    /// Find a peer by UUID
    pub fn find_peer(&self, uuid: &str) -> Option<&Peer> {
        self.peers.iter().find(|p| p.uuid == uuid)
    }

    /// Add a new peer
    pub fn add_peer(&mut self, peer: Peer) {
        self.peers.push(peer.clone());
        let mut controller = self.controller.lock().unwrap();
        let mut updated_peers = controller.get_peers();
        updated_peers.push(peer);
        controller.set_peers(updated_peers);
    }

    /// Remove a peer
    pub fn remove_peer(&mut self, uuid: &str) -> bool {
        if let Some(pos) = self.peers.iter().position(|p| p.uuid == uuid) {
            self.peers.remove(pos);
            let mut controller = self.controller.lock().unwrap();
            controller.set_peers(self.peers.clone());
            true
        } else {
            false
        }
    }

    /// Get network statistics
    pub fn get_stats(&self) -> NetworkStats {
        NetworkStats {
            local_peer_endpoints: self.local_peer.endpoints.len(),
            total_peers: self.peers.len(),
            active_connections: 0, // TODO: Implement connection tracking
        }
    }
}

/// Network statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub local_peer_endpoints: usize,
    pub total_peers: usize,
    pub active_connections: usize,
}

/// Default observer implementation for logging
pub struct LoggingObserver;

impl SocketObserver for LoggingObserver {
    fn on_message_received(&self, message: ChatMessage) {
        println!("🔔 Observer: Message received from {}: {}", message.sender.name, message.text);
    }

    fn on_ack_received(
        &self,
        message_uuid: &str,
        is_read: bool,
        ack_time: chrono::DateTime<chrono::Utc>,
    ) {
        println!("🔔 Observer: ACK received for {} (read: {}) at {}", 
            message_uuid, is_read, ack_time.format("%H:%M:%S"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::Endpoint;

    fn create_test_peer(uuid: &str, name: &str, endpoints: Vec<Endpoint>) -> Peer {
        Peer {
            uuid: uuid.to_string(),
            name: name.to_string(),
            endpoints,
            color: 0,
        }
    }

    #[test]
    fn test_network_engine_creation() {
        let local_peer = create_test_peer("local", "Local Peer", vec![
            Endpoint::Tcp("127.0.0.1:7001".to_string()),
        ]);
        
        let peers = vec![
            create_test_peer("peer1", "Peer 1", vec![
                Endpoint::Tcp("127.0.0.1:7002".to_string()),
            ]),
        ];

        let engine = NetworkEngine::new(local_peer.clone(), peers.clone()).unwrap();
        
        assert_eq!(engine.local_peer().uuid, "local");
        assert_eq!(engine.peers().len(), 1);
        assert!(engine.find_peer("peer1").is_some());
        assert!(engine.find_peer("nonexistent").is_none());
    }

    #[test]
    fn test_peer_management() {
        let local_peer = create_test_peer("local", "Local Peer", vec![]);
        let mut engine = NetworkEngine::new(local_peer, vec![]).unwrap();

        let new_peer = create_test_peer("new", "New Peer", vec![
            Endpoint::Tcp("127.0.0.1:8000".to_string()),
        ]);

        engine.add_peer(new_peer.clone());
        assert_eq!(engine.peers().len(), 1);
        assert!(engine.find_peer("new").is_some());

        let removed = engine.remove_peer("new");
        assert!(removed);
        assert_eq!(engine.peers().len(), 0);
        assert!(engine.find_peer("new").is_none());
    }

    #[test]
    fn test_network_stats() {
        let local_peer = create_test_peer("local", "Local Peer", vec![
            Endpoint::Tcp("127.0.0.1:7001".to_string()),
            Endpoint::Udp("127.0.0.1:7002".to_string()),
        ]);
        
        let peers = vec![
            create_test_peer("peer1", "Peer 1", vec![]),
            create_test_peer("peer2", "Peer 2", vec![]),
        ];

        let engine = NetworkEngine::new(local_peer, peers).unwrap();
        let stats = engine.get_stats();

        assert_eq!(stats.local_peer_endpoints, 2);
        assert_eq!(stats.total_peers, 2);
    }
}
