use crate::network::peer_manager::PeerManager;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub local_peer_endpoints: usize,
    pub total_peers: usize,
    pub active_connections: usize,
}

pub struct NetworkMonitor {
    peer_manager: Arc<Mutex<PeerManager>>,
    // TODO: Add connection tracker
}

impl NetworkMonitor {
    pub fn new(peer_manager: Arc<Mutex<PeerManager>>) -> Self {
        Self { peer_manager }
    }

    pub fn get_stats(&self) -> NetworkStats {
        let peer_manager = self.peer_manager.lock().unwrap();
        NetworkStats {
            local_peer_endpoints: peer_manager.local_peer_endpoints_count(),
            total_peers: peer_manager.total_peers(),
            active_connections: 0, // TODO: Implement connection tracking
        }
    }

    // TODO: Add methods for monitoring network health, connection status, etc.
    pub fn is_healthy(&self) -> bool {
        let peer_manager = self.peer_manager.lock().unwrap();
        peer_manager.local_peer_endpoints_count() > 0
    }
}
