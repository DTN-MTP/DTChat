use serde::Deserialize;
use std::{cell::RefCell, fs, rc::Rc};

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct PeerAttributes {
    pub uuid: String,
    pub name: String,
    pub endpoint: String,
    pub color: u32,
}

impl PeerAttributes {
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

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct RoomAttributes {
    pub name: String,
}

/// New type alias `SharedPeer` to refer to `Peer` via Rc<RefCell<_>>
/// Facilitate shared ownership and mutable access to `Peer` instances across various modules
pub type SharedPeer = Rc<RefCell<PeerAttributes>>;
pub type SharedRoom = Rc<RefCell<RoomAttributes>>;

#[derive(Debug, Deserialize)]
pub struct AppConfigManager {
    pub peer_list: Vec<PeerAttributes>,
    pub room_list: Vec<RoomAttributes>,
}

impl AppConfigManager {
    pub fn load_yaml_from_file(file_path: &str) -> Self {
        let config_str = fs::read_to_string(file_path).expect("Failed to read config file");
        serde_yaml::from_str(&config_str).expect("Failed to parse YAML")
    }

    pub fn shared_peers(&self) -> Vec<SharedPeer> {
        self.peer_list
            .iter()
            .map(|peer| Rc::new(RefCell::new((*peer).clone())))
            .collect()
    }

    pub fn shared_rooms(&self) -> Vec<SharedRoom> {
        self.room_list
            .iter()
            .map(|room| Rc::new(RefCell::new(room.clone())))
            .collect()
    }
}
