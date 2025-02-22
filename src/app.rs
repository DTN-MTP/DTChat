use crate::layout::menu_bar::NavigationItems;
use crate::layout::rooms::message_settings_bar::RoomView;
use crate::layout::ui;
use crate::utils::config::{AppConfigManager, SharedPeer, SharedRoom};
use crate::utils::message::{Message, MessageStatus};
use crate::utils::socket::{
    run_tcp_listener, run_udp_listener, start_tcp_listener, start_udp_listener, TOKIO_RUNTIME,
};
use chrono::{Duration, Local};
use eframe::egui;
use std::sync::{Arc, Mutex};

pub enum AppEvent {
    MessageSent(Message),
    MessageReceived(Message),
}

pub trait ModelObserver: Send + Sync {
    fn on_event(&self, event: &AppEvent);
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

    pub fn notify_observers(&self, event: AppEvent) {
        for obs in &self.observers {
            obs.on_event(&event);
        }
    }

    pub fn add_message(&mut self, msg: Message) {
        self.messages.push(msg.clone());
        self.notify_observers(AppEvent::MessageReceived(msg));
    }

    pub fn send_message(&mut self, app: &mut ChatApp) {
        let forging_sender = &app.message_panel.forging_sender;
        let protocol = forging_sender.lock().unwrap().protocol.clone();
        let endpoint = forging_sender.lock().unwrap().endpoint.clone();
        let text = app.message_panel.message_to_send.clone();
        let socket = match protocol.as_str() {
            "tcp" => crate::utils::socket::create_sending_socket(
                crate::utils::socket::ProtocolType::Tcp,
                &endpoint,
            ),
            #[cfg(feature = "bp")]
            "bp" => crate::utils::socket::create_sending_socket(
                crate::utils::socket::ProtocolType::Bp,
                &endpoint,
            ),
            _ => crate::utils::socket::create_sending_socket(
                crate::utils::socket::ProtocolType::Udp,
                &endpoint,
            ),
        };
        let msg = if let Err(e) = socket.and_then(|mut s| s.send(&text)) {
            Message {
                uuid: "ERR".to_string(),
                response: None,
                sender: Arc::clone(forging_sender),
                text: format!("Socket error: {:?}", e),
                shipment_status: MessageStatus::Sent(String::new()),
            }
        } else {
            Message {
                uuid: "TODO".to_string(),
                response: None,
                sender: Arc::clone(forging_sender),
                text,
                shipment_status: MessageStatus::Sent(Local::now().format("%H:%M:%S").to_string()),
            }
        };
        self.add_message(msg.clone());
        self.notify_observers(AppEvent::MessageSent(msg));
        app.message_panel.message_to_send.clear();
        app.sort_messages();
    }

    pub fn receive_message(&mut self, text: &str, sender: SharedPeer) {
        let now = Local::now().format("%H:%M:%S").to_string();
        let msg = Message {
            uuid: "RCVD".to_string(),
            response: None,
            sender,
            text: text.to_string(),
            shipment_status: MessageStatus::Received(now.clone(), now),
        };
        self.add_message(msg);
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
    pub fn ui(app: &mut crate::app::ChatApp, ctx: &egui::Context) {
        ui::display(app, ctx);
    }
}

impl ModelObserver for ChatView {
    fn on_event(&self, _event: &AppEvent) {}
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
        let view = Arc::new(ChatView {
            model: Arc::clone(&model_arc),
        });
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
        ChatView::ui(self, ctx);
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
