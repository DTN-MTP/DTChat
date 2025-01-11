use crate::layout::header::ConnectionStatus;
use crate::message::{Message, MessagePriority, MessageType};
use crate::ui;
use chrono::Local;
use eframe::egui;

pub struct ChatApp {
    pub messages: Vec<Message>,
    pub message_to_send: String,
    pub send_time: String,
    pub message_priority: MessagePriority,
    pub message_type: MessageType,
    pub local_endpoint: String,
    pub peer_endpoint: String,
    pub connection_status: ConnectionStatus,
}

impl Default for ChatApp {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            message_to_send: String::new(),
            send_time: Local::now().format("%H:%M:%S").to_string(),
            message_priority: MessagePriority::Normal,
            message_type: MessageType::Request,
            peer_endpoint: "ipn:<node_id>.1".to_string(),
            local_endpoint: "ipn:<node_id>.0".to_string(),
            connection_status: ConnectionStatus::Connected,
        }
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::display(self, ctx);
    }
}
