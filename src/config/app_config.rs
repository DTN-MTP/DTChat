use crate::domain::{Peer, Room};
use serde::Deserialize;
use std::fs;

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
