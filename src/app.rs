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
use std::sync::{Arc, Mutex};

pub trait ModelObserver: Send + Sync {
    fn on_message_added(&self, msg: &Message);
}

pub struct ChatModel {
    pub peers: Vec<SharedPeer>,
    pub rooms: Vec<SharedRoom>,
    pub messages: Vec<Message>,
    observers: Vec<Arc<dyn ModelObserver>>,
}

impl ChatModel {
    pub fn new(peers: Vec<SharedPeer>, rooms: Vec<SharedRoom>) -> Self {
        Self {
            peers,
            rooms,
            messages: Vec::new(),
            observers: Vec::new(),
        }
    }
    pub fn add_observer(&mut self, obs: Arc<dyn ModelObserver>) {
        self.observers.push(obs);
    }
    pub fn add_message(&mut self, msg: Message) {
        self.messages.push(msg.clone());
        for obs in &self.observers {
            obs.on_message_added(&msg);
        }
    }
    pub fn sort_messages(&mut self, ctx_peer_uuid: &str) {
        self.messages.sort_by(|a, b| {
            let (tx_a, rx_a) = match &a.shipment_status {
                MessageStatus::Sent(tx) => (tx, tx),
                MessageStatus::Received(tx, rx) => (tx, rx),
            };
            let (tx_b, rx_b) = match &b.shipment_status {
                MessageStatus::Sent(tx) => (tx, tx),
                MessageStatus::Received(tx, rx) => (tx, rx),
            };
            let anchor_a = if a.sender.lock().unwrap().uuid == ctx_peer_uuid {
                rx_a
            } else {
                tx_a
            };
            let anchor_b = if b.sender.lock().unwrap().uuid == ctx_peer_uuid {
                rx_b
            } else {
                tx_b
            };
            anchor_a.cmp(anchor_b)
        });
    }
}

pub struct ChatView {
    pub model: Arc<Mutex<ChatModel>>,
}

impl ChatView {
    pub fn new(model: Arc<Mutex<ChatModel>>) -> Self {
        Self { model }
    }
    pub fn ui(&self, _app: &mut ChatApp, _ctx: &egui::Context) {}
}

impl ModelObserver for ChatView {
    fn on_message_added(&self, _msg: &Message) {}
}

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
    pub model: Arc<Mutex<ChatModel>>,
    pub context_menu: NavigationItems,
    pub message_panel: MessagePanel,
    pub view: Arc<ChatView>,
    pub peers: Vec<SharedPeer>,
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
        let model = ChatModel::new(shared_peers.clone(), shared_rooms.clone());
        let model_arc = Arc::new(Mutex::new(model));
        let view = Arc::new(ChatView::new(Arc::clone(&model_arc)));
        {
            let mut lock = model_arc.lock().unwrap();
            lock.add_observer(Arc::clone(&view) as Arc<dyn ModelObserver>);
        }
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
            model: model_arc,
            view,
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
        self.model.lock().unwrap().sort_messages(&ctx_peer_uuid);
        let sorted = self.model.lock().unwrap().messages.clone();
        self.message_panel.messages = sorted;
    }
}

pub fn init_app() -> Arc<Mutex<ChatApp>> {
    let app = ChatApp::default();
    let app_arc = Arc::new(Mutex::new(app));
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
