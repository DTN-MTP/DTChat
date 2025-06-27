use serde::Deserialize;
use std::fs;

use super::socket::Endpoint;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Peer {
    pub uuid: String,
    pub name: String,
    pub endpoints: Vec<Endpoint>,
    pub color: u32,
}

impl Default for Peer {
    fn default() -> Self {
        Self {
            uuid: "unknown".to_string(),
            name: "Unknown".to_string(),
            endpoints: Vec::new(),
            color: 0,
        }
    }
}

impl Peer {
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
pub struct Room {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AppConfigManager {
    pub peer_list: Vec<Peer>,
    pub local_peer: Peer,
    pub room_list: Vec<Room>,
    pub a_sabr: String,
}

impl AppConfigManager {
    pub fn load_yaml_from_file(file_path: &str) -> Self {
        let config_str = fs::read_to_string(file_path).expect("Failed to read config file");
        serde_yaml::from_str(&config_str).expect("Failed to parse YAML")
    }
}
