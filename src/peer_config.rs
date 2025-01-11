use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Peer {
    //pub name: String,
    pub endpoint: String,
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
