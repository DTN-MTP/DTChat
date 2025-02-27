use crate::layout::menu_bar::NavigationItems;
use crate::layout::rooms::message_settings_bar::RoomView;
use crate::layout::ui;
use crate::utils::config::{AppConfigManager, SharedPeer, SharedRoom};
use crate::utils::message::{Message, MessageStatus};
use crate::utils::socket::{
    create_sending_socket, DefaultSocketController, ProtocolType, SocketController, SocketObserver,
};
use chrono::{Duration, Local};
use eframe::egui;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

pub enum AppEvent {
    SendFailed(Message),
    MessageSent(Message),
    MessageReceived(Message),
}

pub enum SortStrategy {
    Standard,
    Relative(String)
}

fn standard_cmp(a: &Message, b: &Message) -> Ordering {
    let (tx_a, rx_a) = match &a.shipment_status {
        MessageStatus::Sent(tx) => (tx, tx),
        MessageStatus::Received(tx, rx) => (tx, rx),
    };
    let (tx_b, rx_b) = match &b.shipment_status {
        MessageStatus::Sent(tx) => (tx, tx),
        MessageStatus::Received(tx, rx) => (tx, rx),
    };
    tx_a.cmp(tx_b).then(rx_a.cmp(rx_b))
}

fn relative_cmp(a: &Message, b: &Message, ctx_peer_uuid: &str) -> Ordering {
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
    return anchor_a.cmp(anchor_b)
}


pub trait ModelObserver: Send + Sync {
    fn on_event(&self, event: &AppEvent);
}

pub struct ChatModel {
    pub sort_strategy: SortStrategy,
    pub peers: Vec<SharedPeer>,
    pub rooms: Vec<SharedRoom>,
    pub messages: Vec<Message>,
    observers: Vec<Arc<dyn ModelObserver>>,
}

impl ChatModel {
    pub fn new(peers: Vec<SharedPeer>, rooms: Vec<SharedRoom>) -> Self {
        Self {
            sort_strategy: SortStrategy::Standard,
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

    pub fn add_message(&mut self, new_msg: Message) {

        let idx = match &self.sort_strategy {
            SortStrategy::Standard => self.messages
            .binary_search_by(|msg| standard_cmp(msg, &new_msg))
            .unwrap_or_else(|i| i),
            SortStrategy::Relative(ctx_peer_uuid) => self.messages
            .binary_search_by(|msg| relative_cmp(msg, &new_msg, ctx_peer_uuid.as_str()))
            .unwrap_or_else(|i| i),
        };
        self.messages.insert(idx, new_msg.clone());
        self.notify_observers(AppEvent::MessageReceived(new_msg));
    }

    pub fn send_message(&mut self, msg: &mut Message) {
        let protocol = msg.sender.lock().unwrap().protocol.clone();
        let endpoint = msg.sender.lock().unwrap().endpoint.clone();
        let socket = match protocol.as_str() {
            "tcp" => create_sending_socket(ProtocolType::Tcp, &endpoint),
            #[cfg(feature = "bp")]
            "bp" => create_sending_socket(ProtocolType::Bp, &endpoint),
            _ => create_sending_socket(ProtocolType::Udp, &endpoint),
        };
        if let Err(e) = socket.and_then(|mut s| s.send(&msg.text)) {
            eprintln!("Failed to send via TCP/UDP: {:?}", e);
            self.notify_observers(AppEvent::SendFailed(msg.clone()));
            return;
        }
        msg.uuid = "SENT".to_string();
        msg.shipment_status = MessageStatus::Sent(Local::now().format("%H:%M:%S").to_string());
        self.add_message(msg.clone());
        self.notify_observers(AppEvent::MessageSent(msg.clone()));
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

    pub fn sort_messages(&mut self) {

        match &self.sort_strategy{
            SortStrategy::Standard => self.messages.sort_by(|a, b | standard_cmp(a, b)),
            SortStrategy::Relative(for_peer) => self.messages.sort_by(|a, b| relative_cmp(a, b, for_peer.as_str())),
        }
    }
}

impl ModelObserver for ChatModel {
    fn on_event(&self, _event: &AppEvent) {}
}

impl SocketObserver for Mutex<ChatModel> {
    fn on_socket_event(&self, text: &str, sender: SharedPeer) {
        let mut model = self.lock().unwrap();
        model.receive_message(text, sender);
    }
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
    pub peers: Vec<SharedPeer>,
    pub socket_controller: Arc<Mutex<dyn SocketController + Send + Sync>>,
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
        let model = ChatModel::new(shared_peers.clone(), shared_rooms.clone());
        let model_arc = Arc::new(Mutex::new(model));

        let socket_controller =
            DefaultSocketController::init_controller(Arc::clone(&local_peer)).unwrap();
        {
            socket_controller
                .lock()
                .unwrap()
                .add_observer(model_arc.clone() as Arc<dyn SocketObserver + Send + Sync>);
        }
        let app = Self {
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
            socket_controller,
        };
        return app;
    }
}

impl ChatApp {
    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::display(self, ctx);
    }

    pub fn sort_messages(&mut self, peer_name: String) {

            let mut model = self.model.lock().unwrap();
            model.sort_strategy = SortStrategy::Relative(peer_name);
            model.sort_messages();
    }
}

impl ModelObserver for Mutex<ChatApp> {
    fn on_event(&self, event: &AppEvent) {
        // match event {
        //     AppEvent::SendFailed(message) => self.lock().unwrap().message_panel.send_status = Some("Send Error".to_string()),
        //     AppEvent::MessageSent(message) => self.lock().unwrap().message_panel.send_status = Some("Message sent".to_string()),
        //     AppEvent::MessageReceived(message) => self.lock().unwrap().message_panel.send_status = Some("Message received".to_string()),
        // }
    }
}

pub fn init_app() -> Arc<Mutex<ChatApp>> {
    let app =  ChatApp::default();
    let model_arc = app.model.clone();
    let app_arc = Arc::new(Mutex::new(app));

    model_arc.lock().unwrap().add_observer(app_arc.clone() as Arc<dyn ModelObserver>);
    return app_arc;
}
