use crate::layout::menu_bar::NavigationItems;
use crate::layout::rooms::message_settings_bar::RoomView;
use crate::layout::ui::{self, display};
use crate::utils::config::{Peer, Room};
use crate::utils::message::{Message, MessageStatus};
use crate::utils::socket::{
    create_sending_socket, DefaultSocketController, ProtocolType, SocketController, SocketObserver,
};
use chrono::{Duration, Local, Utc};
use eframe::egui;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub enum AppEvent {
    SendFailed(Message),
    MessageSent(Message),
    MessageReceived(Message),
}

#[derive(PartialEq, Eq, Clone)]
pub enum SortStrategy {
    Standard,
    Relative(Peer),
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
    let anchor_a = if a.sender.uuid == ctx_peer_uuid {
        rx_a
    } else {
        tx_a
    };
    let anchor_b = if b.sender.uuid == ctx_peer_uuid {
        rx_b
    } else {
        tx_b
    };
    return anchor_a.cmp(anchor_b);
}

pub struct ChatModel {
    pub sort_strategy: SortStrategy,
    pub localpeer: Peer,
    pub peers: Vec<Peer>,
    pub rooms: Vec<Room>,
    pub messages: Vec<Message>,
    observers: Vec<Arc<Mutex<dyn ModelObserver>>>,
}

impl ChatModel {
    pub fn new(peers: Vec<Peer>, localpeer: Peer, rooms: Vec<Room>) -> Self {
        Self {
            sort_strategy: SortStrategy::Standard,
            localpeer,
            peers,
            rooms,
            messages: Vec::new(),
            observers: Vec::new(),
        }
    }

    pub fn add_observer(&mut self, obs: Arc<Mutex<dyn ModelObserver>>) {
        self.observers.push(obs);
    }

    pub fn notify_observers(&self, event: AppEvent) {
        for obs in &self.observers {
            obs.lock().unwrap().on_event(event.clone());
        }
    }

    pub fn add_message(&mut self, new_msg: Message) {
        let idx = match &self.sort_strategy {
            SortStrategy::Standard => self
                .messages
                .binary_search_by(|msg| standard_cmp(msg, &new_msg))
                .unwrap_or_else(|i| i),
            SortStrategy::Relative(peer) => self
                .messages
                .binary_search_by(|msg| relative_cmp(msg, &new_msg, &peer.uuid.as_str()))
                .unwrap_or_else(|i| i),
        };
        self.messages.insert(idx, new_msg.clone());
        self.notify_observers(AppEvent::MessageReceived(new_msg));
    }

    pub fn send_message(&mut self, text: &str, receiver: Peer) {
        let mut msg = Message {
            uuid: "PLACEHOLDER_CHANGE_THAT".to_string(),
            response: None,
            sender: self.localpeer.clone(),
            text: text.to_string(),
            shipment_status: MessageStatus::Sent(Utc::now()),
        };

        let protocol = receiver.protocol.clone();
        let endpoint = receiver.endpoint.clone();
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
        msg.uuid = "PLACEHOLDER_CHANGE_THAT".to_string();
        self.add_message(msg.clone());
        self.notify_observers(AppEvent::MessageSent(msg.clone()));
    }

    pub fn receive_message(&mut self, text: &str, sender: Peer) {
        let now = Utc::now();
        let msg = Message {
            uuid: "PLACEHOLDER_CHANGE_THAT".to_string(),
            response: None,
            sender,
            text: text.to_string(),
            shipment_status: MessageStatus::Received(now.clone(), now),
        };
        self.add_message(msg);
    }

    pub fn sort_messages(&mut self, strat: SortStrategy) {
        self.sort_strategy = strat;

        match &self.sort_strategy {
            SortStrategy::Standard => self.messages.sort_by(|a, b| standard_cmp(a, b)),
            SortStrategy::Relative(for_peer) => self
                .messages
                .sort_by(|a, b| relative_cmp(a, b, for_peer.uuid.as_str())),
        }
    }
}

impl SocketObserver for Mutex<ChatModel> {
    fn on_socket_event(&self, text: &str, sender: Peer) {
        let mut model = self.lock().unwrap();
        model.receive_message(text, sender);
    }
}

pub struct MessagePanel {
    pub message_view: RoomView,
    pub create_modal_open: bool,
    pub message_to_send: String,
    pub forging_tx_time: String,
    pub forging_rx_time: String,
    pub forging_receiver: Peer,
    pub send_status: Option<String>,
}

pub struct ChatApp {
    pub model_arc: Arc<Mutex<ChatModel>>,
    pub handler_arc: Arc<Mutex<EventHandler>>,
    pub context_menu: NavigationItems,
    pub message_panel: MessagePanel,
}

impl ChatApp {
    pub fn new(model_arc: Arc<Mutex<ChatModel>>, handler_arc: Arc<Mutex<EventHandler>>) -> Self {
        let forging_receiver = model_arc.lock().unwrap().peers[0].clone();
        let app = Self {
            model_arc: model_arc,
            handler_arc: handler_arc,
            context_menu: NavigationItems::default(),
            message_panel: MessagePanel {
                message_view: RoomView::default(),
                create_modal_open: false,
                message_to_send: String::new(),
                forging_tx_time: Local::now().format("%H:%M:%S").to_string(),
                forging_rx_time: (Local::now() + Duration::hours(1))
                    .format("%H:%M:%S")
                    .to_string(),
                forging_receiver,
                send_status: None,
            },
        };
        return app;
    }
}

#[derive(Default)]
pub struct EventHandler {
    pub events: VecDeque<AppEvent>,
    pub ctx: egui::Context,
}

impl EventHandler {
    pub fn new(ctx: egui::Context) -> Self {
        return Self {
            events: VecDeque::new(),
            ctx: ctx,
        };
    }
}

pub trait ModelObserver: Send + Sync {
    fn on_event(&mut self, event: AppEvent);
}

impl ModelObserver for EventHandler {
    fn on_event(&mut self, event: AppEvent) {
        match &event {
            AppEvent::MessageReceived(_message) => self.ctx.request_repaint(),
            _ => (),
        }

        self.events.push_back(event);
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        display(self, ctx);
    }
}
