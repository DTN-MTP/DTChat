use crate::layout::header::ConnectionStatus;
use crate::message::{Message, MessagePriority, MessageType};
use crate::ui;
use chrono::{Duration, Local};
use eframe::egui;

pub struct ChatApp {
    pub messages: Vec<Message>,
    pub message_to_send: String,
    pub sent_by_user: bool,
    pub send_time: String,
    pub receive_time: String,
    pub message_priority: MessagePriority,
    pub message_type: MessageType,
    pub local_endpoint: String,
    pub peer_endpoint: String,
    pub connection_status: ConnectionStatus,
}

impl Default for ChatApp {
    fn default() -> Self {
        let recv_time = Local::now() + Duration::hours(1);
        Self {
            messages: Vec::new(),
            message_to_send: String::new(),
            sent_by_user: true,
            send_time: Local::now().format("%H:%M:%S").to_string(),
            receive_time: recv_time.format("%H:%M:%S").to_string(),
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
