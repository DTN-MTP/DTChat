use serde::Deserialize;
use std::{cell::RefCell, fs, rc::Rc};

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Peer {
    pub uuid: String,
    pub name: String,
    pub endpoint: String,
    pub color: u32,
}

impl Peer {
    pub fn get_color(&self) -> egui::Color32 {
        let color_id = self.color % 3;
        match color_id {
            0 => egui::Color32::GREEN,
            1 => egui::Color32::RED,
            2 => egui::Color32::BLUE,
            _ => egui::Color32::WHITE,
        }
    }
}

/// New type alias to refer to `Peer` via Rc<RefCell<_>>.
pub type SharedPeer = Rc<RefCell<Peer>>;

#[derive(Debug, Deserialize)]
pub struct PeerConfig {
    pub peer_list: Vec<Peer>,
}

impl PeerConfig {
    pub fn load_from_file(file_path: &str) -> Self {
        let config_str = fs::read_to_string(file_path)
            .expect("Failed to read config file");
        serde_yaml::from_str(&config_str)
            .expect("Failed to parse YAML")
    }

    pub fn into_shared_peers(self) -> Vec<SharedPeer> {
        self.peer_list
            .into_iter()
            .map(|peer| Rc::new(RefCell::new(peer)))
            .collect()
    }
}