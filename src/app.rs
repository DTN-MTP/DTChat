use crate::layout::rooms::message_settings_bar::RoomView;
use crate::layout::ui;
use crate::utils::config::{AppConfigManager, SharedPeer, SharedRoom};
use crate::utils::message::MessageStatus;
use crate::utils::socket::{AddressFamily, ChatSocket};
use crate::{layout::menu_bar::NavigationItems, utils::message::Message};
use chrono::{Duration, Local};
use eframe::egui;
use std::rc::Rc;

pub struct MessagePanel {
    pub message_view: RoomView,
    pub create_modal_open: bool,
    pub show_view_from: SharedPeer,
    pub message_to_send: String,
    pub forging_sender: SharedPeer,
    pub forging_tx_time: String,
    pub forging_rx_time: String,
    pub rooms: Vec<SharedRoom>,
    pub messages: Vec<Message>,
}

pub struct ChatApp {
    pub peers: Vec<SharedPeer>,
    pub context_menu: NavigationItems,
    pub message_panel: MessagePanel,
    pub socket: Option<ChatSocket>,
}

impl Default for ChatApp {
    fn default() -> Self {
        // 1) Load YAML to PeerConfig, then convert to Vec<SharedPeer>.
        let config = AppConfigManager::load_yaml_from_file("database.yaml");
        let shared_peers = config.shared_peers();
        let shared_rooms = config.shared_rooms();

        let forging_sender = Rc::clone(&shared_peers[0]);
        let recv_time = Local::now() + Duration::hours(1);

        ChatApp {
            peers: shared_peers,
            context_menu: NavigationItems::default(),
            message_panel: MessagePanel {
                message_view: RoomView::default(),
                create_modal_open: false,
                show_view_from: Rc::clone(&forging_sender),
                message_to_send: String::new(),
                rooms: shared_rooms,
                forging_sender,
                forging_tx_time: Local::now().format("%H:%M:%S").to_string(),
                forging_rx_time: recv_time.format("%H:%M:%S").to_string(),
                messages: Vec::new(),
            },
            socket: None,
        }
    }
}

impl ChatApp {
    pub fn sort_messages(&mut self) {
        let ctx_peer_uuid = self.message_panel.show_view_from.borrow().uuid.clone();

        self.message_panel.messages.sort_by(|msg_a, msg_b| {
            let (tx_time_a, rx_time_a) = match &msg_a.shipment_status {
                MessageStatus::Sent(tx_time) => (tx_time, tx_time),
                MessageStatus::Received(tx_time, rx_time) => (tx_time, rx_time),
            };

            let anchor_a = if msg_a.sender.borrow().uuid == ctx_peer_uuid {
                rx_time_a
            } else {
                tx_time_a
            };

            let (tx_time_b, rx_time_b) = match &msg_b.shipment_status {
                MessageStatus::Sent(tx_time) => (tx_time, tx_time),
                MessageStatus::Received(tx_time, rx_time) => (tx_time, rx_time),
            };

            let anchor_b = if msg_b.sender.borrow().uuid == ctx_peer_uuid {
                rx_time_b
            } else {
                tx_time_b
            };

            anchor_a.cmp(&anchor_b)
        });
    }

    pub fn try_connect_socket(&mut self) -> std::io::Result<()> {
        if self.socket.is_none() {
            let mut socket = ChatSocket::new(AddressFamily::IPv4)?;
            // For BP protocol testing: let mut socket = ChatSocket::new(AddressFamily::BP(42))?;

            if let Ok(()) = socket.connect() {
                println!("Socket connected successfully");
                self.socket = Some(socket);
            }
        }
        Ok(())
    }

    pub fn check_socket_messages(&mut self) {
        if let Some(socket) = &self.socket {
            if let Ok(Some(received)) = socket.receive() {
                self.message_panel.messages.push(Message {
                    uuid: String::from("RECEIVED"),
                    response: None,
                    sender: Rc::clone(&self.message_panel.forging_sender),
                    text: received,
                    shipment_status: MessageStatus::Received(
                        Local::now().format("%H:%M:%S").to_string(),
                        Local::now().format("%H:%M:%S").to_string(),
                    ),
                });
                self.sort_messages();
            }
        }
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_socket_messages();
        ui::display(self, ctx);
    }
}
