use crate::app::ChatApp;
use crate::message::{ContextSender, Message};
use eframe::egui;
use egui::{vec2, Color32, ComboBox, Rounding, TextEdit};

pub struct FooterLayout {}

impl FooterLayout {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label("Sender:");
            ComboBox::from_id_salt("context_sender")
                .selected_text(format!("{:?}", app.ctx_sender))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.ctx_sender, ContextSender::Me, "Me");
                    ui.selectable_value(&mut app.ctx_sender, ContextSender::Peer, "Peer");
                });
            ui.add_space(4.0);
            ui.label("Send Time:");
            ui.add(TextEdit::singleline(&mut app.send_time).desired_width(100.0));
            ui.label("Receive Time:");
            ui.add(TextEdit::singleline(&mut app.receive_time).desired_width(100.0));
        });

        ui.separator();
        ui.add_space(4.0);

        let mut send_message = false;
        ui.horizontal(|ui| {
            let text_edit = TextEdit::singleline(&mut app.message_to_send)
                .hint_text("Write a message...")
                .desired_width(ui.available_width() - 200.0);

            let response = ui.add(text_edit);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                send_message = true;
            }
            if ui
                .add(
                    egui::Button::new("Send")
                        .fill(Color32::from_rgb(0, 120, 215))
                        .rounding(Rounding::same(2.0))
                        .min_size(vec2(65.0, 10.0)),
                )
                .clicked()
            {
                send_message = true;
            }
        });

        if send_message && !app.message_to_send.trim().is_empty() {
            Message::send(app);
        }
        ui.add_space(10.0);
    }
}
