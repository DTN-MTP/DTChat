use crate::message::{ContextSender, ContextView, Message};
use crate::ui;
use chrono::{Duration, Local};
use eframe::egui;
use serde_yaml::Number;

pub struct ChatApp {
    pub message_id: Number,
    pub messages: Vec<Message>,
    pub message_to_send: String,
    pub send_time: String,
    pub ctx_view: ContextView,
    pub ctx_sender: ContextSender,
    pub view_to_display: ViewToDisplay,
    pub receive_time: String,
    pub peer_endpoint: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ViewToDisplay {
    Table,
    LinearGraph,
    Tchat,
}

impl Default for ChatApp {
    fn default() -> Self {
        let recv_time = Local::now() + Duration::hours(1);

        Self {
            message_id: Number::from(0),
            messages: Vec::new(),
            message_to_send: String::new(),
            send_time: Local::now().format("%H:%M:%S").to_string(),
            receive_time: recv_time.format("%H:%M:%S").to_string(),
            ctx_view: ContextView::Me,
            ctx_sender: ContextSender::Me,
            peer_endpoint: "ipn:<node_id>.1".to_string(),
            view_to_display: ViewToDisplay::Tchat,
        }
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ui::display(self, ctx);
    }
}
