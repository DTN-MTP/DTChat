use crate::domain::{ChatMessage, Peer};
use crate::network::{
    message_router::MessageRouter,
    monitor::{NetworkMonitor, NetworkStats},
    peer_manager::PeerManager,
    socket::{NetworkEventController, NetworkEventManager, SocketObserver},
    Endpoint, NetworkResult,
};
use std::sync::{Arc, Mutex};

pub struct NetworkEngine {
    controller: Arc<Mutex<NetworkEventManager>>,
    peer_manager: Arc<Mutex<PeerManager>>,
    message_router: MessageRouter,
    network_monitor: NetworkMonitor,
}

impl NetworkEngine {
    pub fn new(local_peer: Peer, peers: Vec<Peer>) -> NetworkResult<Self> {
        let peer_manager = Arc::new(Mutex::new(PeerManager::new(
            local_peer.clone(),
            peers.clone(),
        )));
        let controller = NetworkEventManager::init_controller(local_peer, peers)?;
        let message_router = MessageRouter::new(peer_manager.clone());
        let network_monitor = NetworkMonitor::new(peer_manager.clone());

        Ok(Self {
            controller,
            peer_manager,
            message_router,
            network_monitor,
        })
    }

    pub fn add_observer(&self, observer: Arc<dyn SocketObserver>) {
        let mut controller = self.controller.lock().unwrap();
        controller.add_observer(observer);
    }

    // Delegation methods to maintain the same API
    pub fn send_message_to_peer(
        &self,
        message: &ChatMessage,
        peer_uuid: &str,
    ) -> NetworkResult<()> {
        self.message_router.send_message_to_peer(message, peer_uuid)
    }

    pub fn send_message_to_endpoint(
        &self,
        message: &ChatMessage,
        endpoint: &Endpoint,
    ) -> NetworkResult<()> {
        self.message_router
            .send_message_to_endpoint(message, endpoint)
    }

    pub fn send_ack(
        &self,
        original_message: &ChatMessage,
        target_endpoint: &Endpoint,
    ) -> NetworkResult<()> {
        self.message_router
            .send_ack(original_message, target_endpoint)
    }

    pub fn local_peer(&self) -> Peer {
        self.peer_manager.lock().unwrap().local_peer().clone()
    }

    pub fn peers(&self) -> Vec<Peer> {
        self.peer_manager.lock().unwrap().get_peers_clone()
    }

    pub fn find_peer(&self, uuid: &str) -> Option<Peer> {
        self.peer_manager.lock().unwrap().find_peer(uuid).cloned()
    }

    pub fn add_peer(&self, peer: Peer) {
        let mut peer_manager = self.peer_manager.lock().unwrap();
        peer_manager.add_peer(peer.clone());

        // Update controller with new peers
        let mut controller = self.controller.lock().unwrap();
        controller.set_peers(peer_manager.get_peers_clone());
    }

    pub fn remove_peer(&self, uuid: &str) -> bool {
        let mut peer_manager = self.peer_manager.lock().unwrap();
        let result = peer_manager.remove_peer(uuid);

        if result {
            // Update controller with updated peers
            let mut controller = self.controller.lock().unwrap();
            controller.set_peers(peer_manager.get_peers_clone());
        }

        result
    }

    pub fn get_stats(&self) -> NetworkStats {
        self.network_monitor.get_stats()
    }

    pub fn is_healthy(&self) -> bool {
        self.network_monitor.is_healthy()
    }
}
