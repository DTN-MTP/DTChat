use crate::app::ChatApp;
use crate::message::{Message, MessagePriority, MessageType};
use chrono::Local;
use eframe::egui;

pub struct FooterLayout {}

impl FooterLayout {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&self, app: &mut ChatApp, ui: &mut egui::Ui) {
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Send Time:");
            ui.add_enabled(false, egui::TextEdit::singleline(&mut app.send_time));

            ui.label("Priority:");
            egui::ComboBox::from_id_salt("Priority")
                .selected_text(format!("{:?}", app.message_priority))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.message_priority, MessagePriority::Low, "Low");
                    ui.selectable_value(
                        &mut app.message_priority,
                        MessagePriority::Normal,
                        "Normal",
                    );
                    ui.selectable_value(&mut app.message_priority, MessagePriority::High, "High");
                });

            ui.label("Type:");
            egui::ComboBox::from_id_salt("Type")
                .selected_text(format!("{:?}", app.message_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.message_type, MessageType::Request, "Request");
                    ui.selectable_value(&mut app.message_type, MessageType::Response, "Response");
                    ui.selectable_value(&mut app.message_type, MessageType::Code, "Code");
                });
        });

        ui.add_space(4.0);

        let mut send_message = false;
        ui.horizontal(|ui| {
            let text_edit = egui::TextEdit::singleline(&mut app.message_to_send)
                .hint_text("Write a message...")
                .desired_width(ui.available_width() - 200.0);

            let response = ui.add(text_edit);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                send_message = true;
            }
            if ui
                .add(
                    egui::Button::new("Send")
                        .fill(egui::Color32::from_rgb(0, 120, 215))
                        .rounding(egui::Rounding::same(2.0))
                        .min_size(egui::vec2(65.0, 10.0)),
                )
                .clicked()
            {
                send_message = true;
            }
        });

        if send_message && !app.message_to_send.trim().is_empty() {
            app.send_time = Local::now().format("%H:%M:%S").to_string();
            Message::send(app);
        }
        ui.add_space(10.0);
    }
}
