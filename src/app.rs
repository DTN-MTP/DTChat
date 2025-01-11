use chrono::{Duration, Local};
use eframe::egui;
use crate::message::Message;
use crate::ui;

pub struct ChatApp {
    pub messages: Vec<Message>,
    pub input_text: String,
    pub sent_by_user: bool,
    pub send_time: String,
    pub receive_time: String,
}

impl Default for ChatApp {
    fn default() -> Self {
        let recv_time = Local::now() + Duration::hours(1);
        Self {
            messages: Vec::new(),
            input_text: String::new(),
            sent_by_user: true,
            send_time: Local::now().format("%H:%M:%S").to_string(),
            receive_time: recv_time.format("%H:%M:%S").to_string(),
        }
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::update_ui(self, ctx);
    }
}