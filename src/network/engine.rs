use crate::network::{
    socket::{NetworkEventManager, GenericSocket, NetworkEventController, SocketObserver},
    Endpoint, NetworkError, NetworkResult,
};
use crate::network::protocols::ack;
use crate::domain::{Peer, ChatMessage};
use std::sync::{Arc, Mutex};

/// Network engine for managing connections and message routing
pub struct NetworkEngine {
    controller: Arc<Mutex<NetworkEventManager>>,
    local_peer: Peer,
    peers: Vec<Peer>,
}

impl NetworkEngine {
    /// Create a new NetworkEngine instance
    pub fn new(local_peer: Peer, peers: Vec<Peer>) -> NetworkResult<Self> {
        let controller = NetworkEventManager::init_controller(local_peer.clone(), peers.clone())?;
        
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