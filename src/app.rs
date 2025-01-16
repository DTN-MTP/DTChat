

use crate::message::Message;
use crate::peer_config::Peer;
use crate::peer_config::PeerConfig;
use crate::ui;
use chrono::{Duration, Local};
use eframe::egui;

pub struct ChatApp {
    // Data
    pub messages: Vec<Message>,
    pub peers: Vec<Peer>,
    pub show_view_from: Peer,

    pub message_to_send: String,
    pub forging_sender: Peer,
    pub forging_tx_time: String,
    pub forging_rx_time: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ViewToDisplay {
    Table,
    LinearGraph,
    Tchat,
}

impl Default for ChatApp {
    fn default() -> Self {
        let recv_time = Local::now() + Duration::hours(1);
        let peers = PeerConfig::load_from_file("peer-config.yaml").peer_list;
        let forging_sender = peers[0].clone();

        Self {
            messages: Vec::new(),
            peers: peers,
            show_view_from: forging_sender.clone(),
            message_to_send: String::new(),
            forging_sender,
            forging_tx_time: Local::now().format("%H:%M:%S").to_string(),
            forging_rx_time: recv_time.format("%H:%M:%S").to_string(),
        }
    }
}

impl ChatApp {
    pub fn sort_messages(&mut self) {
        let ctx_peer_uuid = self.show_view_from.uuid.clone();

        self.messages.sort_by(|msg_a, msg_b| {
            let (tx_time, rx_time) = match &msg_a.shipment_status {
                crate::message::MessageStatus::Sent(tx_time) => (tx_time, tx_time),
                crate::message::MessageStatus::Received(tx_time, rx_time) => (tx_time, rx_time),
            };
            let anchor_a = if msg_a.sender.uuid == ctx_peer_uuid {
                rx_time
            } else {
                tx_time
            };

            let (tx_time, rx_time) = match &msg_b.shipment_status {
                crate::message::MessageStatus::Sent(tx_time) => (tx_time, tx_time),
                crate::message::MessageStatus::Received(tx_time, rx_time) => (tx_time, rx_time),
            };
            let anchor_b = if msg_b.sender.uuid == ctx_peer_uuid {
                rx_time
            } else {
                tx_time
            };

            anchor_a.cmp(&anchor_b)
        });
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::display(self, ctx);
    }
}
