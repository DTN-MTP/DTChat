use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, PartialEq, Eq, Clone)]
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
            0 => return egui::Color32::GREEN,
            1 => return egui::Color32::RED,
            2 => return egui::Color32::BLUE,
            _ => return egui::Color32::WHITE,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PeerConfig {
    pub peer_list: Vec<Peer>,
}

impl PeerConfig {
    pub fn load_from_file(file_path: &str) -> Self {
        let config_str = fs::read_to_string(file_path).expect("Failed to read config file");
        serde_yaml::from_str(&config_str).expect("Failed to parse YAML")
    }
}
