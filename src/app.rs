use crate::layout::menu_bar::NavigationItems;
use crate::layout::rooms::message_settings_bar::RoomView;
use crate::layout::ui::display;
use crate::utils::prediction_config::prediction_config;
use crate::utils::config::{Peer, Room};
use crate::utils::message::{ChatMessage, MessageStatus};
use crate::utils::socket::SocketObserver;
use chrono::{Duration, Local, DateTime, Utc};
use eframe::egui;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub enum AppEvent {
    MessageError(String),
    MessageSent(String),
    MessageReceived(String),
}

#[derive(PartialEq, Eq, Clone)]
pub enum SortStrategy {
    Standard,
    Relative(Peer),
}

fn standard_cmp(a: &ChatMessage, b: &ChatMessage) -> Ordering {
    let (tx_a, rx_a) = match &a.shipment_status {
        MessageStatus::Sent(tx,rx) => (tx, tx),
        MessageStatus::Received(tx, rx) => (tx, rx),
    };
    let (tx_b, rx_b) = match &b.shipment_status {
        MessageStatus::Sent(tx,rx) => (tx, tx),
        MessageStatus::Received(tx, rx) => (tx, rx),
    };
    tx_a.cmp(tx_b).then(rx_a.cmp(rx_b))
}

fn relative_cmp(a: &ChatMessage, b: &ChatMessage, ctx_peer_uuid: &str) -> Ordering {
    let (tx_a, rx_a) = match &a.shipment_status {
        MessageStatus::Sent(tx,rx) => (tx, tx),
        MessageStatus::Received(tx, rx) => (tx, rx),
    };
    let (tx_b, rx_b) = match &b.shipment_status {
        MessageStatus::Sent(tx,rx) => (tx, tx),
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
    pub messages: Vec<ChatMessage>,
    observers: Vec<Arc<Mutex<dyn ModelObserver>>>,
    pub prediction_config : Option<prediction_config>
}

pub enum MessageDirection {
    Sent,
    Received,
}

impl ChatModel {
    pub fn new(peers: Vec<Peer>, localpeer: Peer, rooms: Vec<Room>,  prediction_config: Option<prediction_config>) -> Self {
        Self {
            sort_strategy: SortStrategy::Standard,
            localpeer,
            peers,
            rooms,
            messages: Vec::new(),
            observers: Vec::new(),
            prediction_config
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

    pub fn add_message(&mut self, new_msg: ChatMessage, direction: MessageDirection) {
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

        let event = match direction {
            MessageDirection::Sent => AppEvent::MessageSent("Message sent.".to_string()),
            MessageDirection::Received => {
                AppEvent::MessageReceived(format!("New message from {}", new_msg.sender.name))
            }
        };
        self.notify_observers(event);
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

    /// Update message status when ACK is received
    pub fn update_message_with_ack(&mut self, message_uuid: &str, is_read: bool, ack_time: DateTime<Utc>) -> bool {
        for message in &mut self.messages {
            if message.uuid == message_uuid {
                message.update_with_ack(is_read, ack_time);
                return true;
            }
        }
        false // Message not found
    }
}

impl SocketObserver for Mutex<ChatModel> {
    fn on_socket_event(&self, message: ChatMessage) {
        let mut model = self.lock().unwrap();

        #[cfg(feature = "delayed_ack")] {
            socket::TOKIO_RUNTIME.spawn(async move {
                use crate::utils::ack::AckConfig;
                let config =  AckConfig::default();
                sleep(Duration::from_millis(config.delay_duration_ms)).await;

                let mut msg_clone = message.clone();
                match msg_clone.shipment_status {
                    MessageStatus::Sent(tx,_)| MessageStatus::Received(tx, _) => msg_clone.shipment_status = MessageStatus::Received(tx, chrono::Utc::now()),
                }
                model.add_message(msg_clone, MessageDirection::Received);
            });
        }
        #[cfg(not(feature = "delayed_ack"))]
        model.add_message(message, MessageDirection::Received);
    }

    fn on_ack_received(&self, message_uuid: &str, is_read: bool, ack_time: chrono::DateTime<chrono::Utc>) {
        let mut model = self.lock().unwrap();
        if model.update_message_with_ack(message_uuid, is_read, ack_time) {
            println!("Updated message {} with ACK (read: {})", message_uuid, is_read);
            // Trigger UI update
            model.notify_observers(AppEvent::MessageSent("Message status updated".to_string()));
        } else {
            println!("ACK received for unknown message: {}", message_uuid);
        }
    }
}

pub struct MessagePanel {
    pub message_view: RoomView,
    pub create_modal_open: bool,
    pub message_to_send: String,
    pub forging_receiver: Peer,
    pub send_status: Option<String>,
    pub pbat_enabled : bool,
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
                forging_receiver,
                send_status: None,
                pbat_enabled : false,
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
