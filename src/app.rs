use crate::layout::rooms::message_settings_bar::RoomView;
use crate::layout::ui;
use crate::utils::config::{AppConfigManager, SharedPeer, SharedRoom};
use crate::utils::message::MessageStatus;
use crate::utils::socket::{
    run_tcp_listener, run_udp_listener, start_tcp_listener, start_udp_listener, TOKIO_RUNTIME,
};
use crate::{layout::menu_bar::NavigationItems, utils::message::Message};
use chrono::{Duration, Local};
use eframe::egui;
use std::sync::Arc;

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
    pub send_status: Option<String>,
}

pub struct ChatApp {
    pub peers: Vec<SharedPeer>,
    pub context_menu: NavigationItems,
    pub message_panel: MessagePanel,
}

impl Default for ChatApp {
    fn default() -> Self {
        let config = AppConfigManager::load_yaml_from_file("database.yaml");
        let shared_peers = config.shared_peers();
        let shared_rooms = config.shared_rooms();
        let local_peer = shared_peers
            .iter()
            .find(|p| p.lock().unwrap().name == "local peer")
            .unwrap()
            .clone();
        let forging_sender = Arc::clone(&local_peer);
        let recv_time = Local::now() + Duration::hours(1);
        let _udp_listener =
            TOKIO_RUNTIME.block_on(async { start_udp_listener("127.0.0.1:7000").await.unwrap() });
        let _tcp_listener =
            TOKIO_RUNTIME.block_on(async { start_tcp_listener("127.0.0.1:7001").await.unwrap() });
        ChatApp {
            peers: shared_peers,
            context_menu: NavigationItems::default(),
            message_panel: MessagePanel {
                message_view: RoomView::default(),
                create_modal_open: false,
                show_view_from: Arc::clone(&local_peer),
                message_to_send: String::new(),
                rooms: shared_rooms,
                forging_sender,
                forging_tx_time: Local::now().format("%H:%M:%S").to_string(),
                forging_rx_time: recv_time.format("%H:%M:%S").to_string(),
                messages: Vec::new(),
                send_status: None,
            },
        }
    }
}

impl ChatApp {
    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::display(self, ctx);
    }

    pub fn sort_messages(&mut self) {
        let ctx_peer_uuid = self
            .message_panel
            .show_view_from
            .lock()
            .unwrap()
            .uuid
            .clone();
        self.message_panel.messages.sort_by(|msg_a, msg_b| {
            let (tx_time_a, rx_time_a) = match &msg_a.shipment_status {
                MessageStatus::Sent(tx) => (tx, tx),
                MessageStatus::Received(tx, rx) => (tx, rx),
            };
            let anchor_a = if msg_a.sender.lock().unwrap().uuid == ctx_peer_uuid {
                rx_time_a
            } else {
                tx_time_a
            };
            let (tx_time_b, rx_time_b) = match &msg_b.shipment_status {
                MessageStatus::Sent(tx) => (tx, tx),
                MessageStatus::Received(tx, rx) => (tx, rx),
            };
            let anchor_b = if msg_b.sender.lock().unwrap().uuid == ctx_peer_uuid {
                rx_time_b
            } else {
                tx_time_b
            };
            anchor_a.cmp(anchor_b)
        });
    }
}

pub fn init_app() -> Arc<std::sync::Mutex<ChatApp>> {
    let app = ChatApp::default();
    let app_arc = Arc::new(std::sync::Mutex::new(app));
    let udp_socket =
        TOKIO_RUNTIME.block_on(async { start_udp_listener("127.0.0.1:7000").await.unwrap() });
    let tcp_listener =
        TOKIO_RUNTIME.block_on(async { start_tcp_listener("127.0.0.1:7001").await.unwrap() });
    {
        let app_clone = Arc::clone(&app_arc);
        let local_peer_clone = Arc::clone(&app_arc.lock().unwrap().message_panel.show_view_from);
        TOKIO_RUNTIME.spawn(async move {
            run_udp_listener(udp_socket, app_clone, local_peer_clone)
                .await
                .unwrap();
        });
    }
    {
        let app_clone = Arc::clone(&app_arc);
        let local_peer_clone = Arc::clone(&app_arc.lock().unwrap().message_panel.show_view_from);
        TOKIO_RUNTIME.spawn(async move {
            run_tcp_listener(tcp_listener, app_clone, local_peer_clone)
                .await
                .unwrap();
        });
    }
    app_arc
}
