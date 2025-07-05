use crate::network::Endpoint;
use serde::Deserialize;

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
