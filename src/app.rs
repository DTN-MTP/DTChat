use crate::message::Message;
use crate::peer_config::{PeerConfig, SharedPeer};
use crate::ui;
use chrono::{Duration, Local};
use eframe::egui;
use std::rc::Rc; 

pub struct ChatApp {
    pub messages: Vec<Message>,
    pub peers: Vec<SharedPeer>,
    pub show_view_from: SharedPeer,

    pub message_to_send: String,
    pub forging_sender: SharedPeer,
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
        // 1) Load YAML to PeerConfig, then convert to Vec<SharedPeer>.
        let config = PeerConfig::load_from_file("peer-config.yaml");
        let shared_peers = config.into_shared_peers();
        let forging_sender = Rc::clone(&shared_peers[0]);
        let recv_time = Local::now() + Duration::hours(1);

        ChatApp {
            messages: Vec::new(),
            peers: shared_peers,
            show_view_from: Rc::clone(&forging_sender),
            message_to_send: String::new(),
            forging_sender,
            forging_tx_time: Local::now().format("%H:%M:%S").to_string(),
            forging_rx_time: recv_time.format("%H:%M:%S").to_string(),
        }
    }
}

impl ChatApp {
    pub fn sort_messages(&mut self) {
        let ctx_peer_uuid = self.show_view_from.borrow().uuid.clone();

        self.messages.sort_by(|msg_a, msg_b| {
            let (tx_time_a, rx_time_a) = match &msg_a.shipment_status {
                crate::message::MessageStatus::Sent(tx_time) => (tx_time, tx_time),
                crate::message::MessageStatus::Received(tx_time, rx_time) => (tx_time, rx_time),
            };

            let anchor_a = if msg_a.sender.borrow().uuid == ctx_peer_uuid {
                rx_time_a
            } else {
                tx_time_a
            };

            let (tx_time_b, rx_time_b) = match &msg_b.shipment_status {
                crate::message::MessageStatus::Sent(tx_time) => (tx_time, tx_time),
                crate::message::MessageStatus::Received(tx_time, rx_time) => (tx_time, rx_time),
            };

            let anchor_b = {
                let sender_uuid_b = msg_b.sender.borrow().uuid.clone();
                if sender_uuid_b == ctx_peer_uuid {
                    rx_time_b
                } else {
                    tx_time_b
                }
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
