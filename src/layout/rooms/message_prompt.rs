use crate::app::ChatApp;
use crate::utils::colors::COLORS;
use crate::utils::message::{ChatMessage, MessageStatus};
use chrono::Local;
use eframe::egui;
use egui::{vec2, CornerRadius, TextEdit};

pub struct MessagePrompt {}

impl MessagePrompt {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        app.handler_arc
            .lock()
            .unwrap()
            .events
            .retain(|event| match event {
                crate::app::AppEvent::SendFailed(_message) => {
                    app.message_panel.send_status = Some("Failed to send message.".to_string());
                    false
                }
                crate::app::AppEvent::MessageSent(_message) => {
                    app.message_panel.send_status = Some("Message Sent.".to_string());
                    false
                }
                crate::app::AppEvent::MessageReceived(_message) => {
                    app.message_panel.send_status = Some("Message received".to_string());
                    false
                }
                _ => true,
            });

        ui.add_space(4.0);
        let mut send_message = false;
        ui.horizontal(|ui| {
            let text_edit = TextEdit::singleline(&mut app.message_panel.message_to_send)
                .hint_text("Write a message...")
                .desired_width(ui.available_width() - 200.0);
            let response = ui.add(text_edit);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                send_message = true;
                response.request_focus();
            }
            if ui
                .add(
                    egui::Button::new("Send")
                        .fill(COLORS[2])
                        .corner_radius(CornerRadius::same(2))
                        .min_size(vec2(65.0, 10.0)),
                )
                .clicked()
            {
                send_message = true;
            }
        });
        if send_message && !app.message_panel.message_to_send.trim().is_empty() {
            let forging_receiver = app.message_panel.forging_receiver.clone();
            if forging_receiver.name == "local peer" {
                app.message_panel.send_status =
                    Some("Cannot send message to local peer".to_string());
            } else {
                let text = app.message_panel.message_to_send.clone();
                let mut model = app.model_arc.lock().unwrap();
                model.send_message(&text, forging_receiver.clone());
                app.message_panel.message_to_send.clear();
            }
        }
        ui.add_space(4.0);
    }
}
