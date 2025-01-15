use crate::message::{Message, MessageType, SendByUser};
use crate::ui;
use chrono::{Duration, Local};
use eframe::egui;

pub struct ChatApp {
    pub messages: Vec<Message>,
    pub message_to_send: String,
    pub send_time: String,
    pub ctx_sender_app: SendByUser,
    pub receive_time: String,
    pub message_type: MessageType,
    pub peer_endpoint: String,
}

impl Default for ChatApp {
    fn default() -> Self {
        let recv_time = Local::now() + Duration::hours(1);

        Self {
            messages: Vec::new(),
            message_to_send: String::new(),
            send_time: Local::now().format("%H:%M:%S").to_string(),
            receive_time: recv_time.format("%H:%M:%S").to_string(),
            message_type: MessageType::Message,
            ctx_sender_app: SendByUser::Earth,
            peer_endpoint: "ipn:<node_id>.1".to_string(),
        }
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::display(self, ctx);
    }
}
