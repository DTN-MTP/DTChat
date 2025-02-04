use serde::Deserialize;
use std::{
    fs,
    sync::{Arc, Mutex},
};

fn default_protocol() -> String {
    "udp".to_string()
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct PeerAttributes {
    pub uuid: String,
    pub name: String,
    pub endpoint: String,
    pub color: u32,
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

impl PeerAttributes {
    pub fn get_color(&self) -> egui::Color32 {
        let color_id = self.color % 4;
        match color_id {
            0 => egui::Color32::GREEN,
            1 => egui::Color32::RED,
            2 => egui::Color32::BLUE,
            3 => egui::Color32::YELLOW,
            _ => egui::Color32::WHITE,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
pub struct RoomAttributes {
    pub name: String,
}

pub type SharedPeer = Arc<Mutex<PeerAttributes>>;
pub type SharedRoom = Arc<Mutex<RoomAttributes>>;

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
            .map(|p| Arc::new(Mutex::new(p.clone())))
            .collect()
    }

    pub fn shared_rooms(&self) -> Vec<SharedRoom> {
        self.room_list
            .iter()
            .map(|r| Arc::new(Mutex::new(r.clone())))
            .collect()
    }
}
