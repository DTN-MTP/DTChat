use crate::app::ChatApp;
use crate::utils::colors::COLORS;
use crate::utils::message::{Message, MessageStatus};
use chrono::Local;
use eframe::egui;
use egui::{vec2, Rounding, TextEdit};

pub struct MessagePrompt {}

impl MessagePrompt {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let mut send_message = false;
        ui.horizontal(|ui| {
            let text_edit = TextEdit::singleline(&mut app.message_panel.message_to_send)
                .hint_text("Write a message...")
                .desired_width(ui.available_width() - 200.0);
            let response = ui.add(text_edit);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                send_message = true;
            }
            if ui
                .add(
                    egui::Button::new("Send")
                        .fill(COLORS[2])
                        .rounding(Rounding::same(2.0))
                        .min_size(vec2(65.0, 10.0)),
                )
                .clicked()
            {
                send_message = true;
            }
        });
        if send_message && !app.message_panel.message_to_send.trim().is_empty() {
            if app.message_panel.forging_sender.lock().unwrap().name == "local peer" {
                app.message_panel.send_status =
                    Some("Cannot send message using local peer".to_string());
            } else {
                let text = app.message_panel.message_to_send.clone();
                let forging_sender = app.message_panel.forging_sender.clone();
                let mut new_msg = Message {
                    uuid: "NEW".to_string(),
                    response: None,
                    sender: forging_sender,
                    text,
                    shipment_status: MessageStatus::Sent(
                        Local::now().format("%H:%M:%S").to_string(),
                    ),
                };
                {
                    let mut model = app.model.lock().unwrap();
                    match model.send_message(&mut new_msg) {
                        Ok(_) => {
                            model.add_message(new_msg);
                        }
                        Err(_e) => {
                            app.message_panel.send_status = Some("Connection refused".to_string());
                        }
                    }
                }
                app.message_panel.message_to_send.clear();
                app.sort_messages();
            }
        }
        ui.add_space(4.0);
    }
}
