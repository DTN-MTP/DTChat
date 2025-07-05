use crate::domain::Peer;

#[derive(Debug, Clone)]
pub struct PeerManager {
    peers: Vec<Peer>,
    local_peer: Peer,
}

impl PeerManager {
    pub fn new(local_peer: Peer, peers: Vec<Peer>) -> Self {
        Self { peers, local_peer }
    }

    pub fn add_peer(&mut self, peer: Peer) {
        if !self.peers.iter().any(|p| p.uuid == peer.uuid) {
            self.peers.push(peer);
        }
    }

    pub fn remove_peer(&mut self, uuid: &str) -> bool {
        if let Some(pos) = self.peers.iter().position(|p| p.uuid == uuid) {
            self.peers.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn find_peer(&self, uuid: &str) -> Option<&Peer> {
        self.peers.iter().find(|p| p.uuid == uuid)
    }

    pub fn local_peer(&self) -> &Peer {
        &self.local_peer
    }

    pub fn get_peers_clone(&self) -> Vec<Peer> {
        self.peers.clone()
    }

    pub fn total_peers(&self) -> usize {
        self.peers.len()
    }

    pub fn local_peer_endpoints_count(&self) -> usize {
        self.local_peer.endpoints.len()
    }
}
