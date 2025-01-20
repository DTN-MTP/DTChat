use crate::{
    app::ChatApp,
    utils::{colors::BLUE, message::Message},
};
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
                        .fill(BLUE)
                        .rounding(Rounding::same(2.0))
                        .min_size(vec2(65.0, 10.0)),
                )
                .clicked()
            {
                send_message = true;
            }
        });

        if send_message && !app.message_panel.message_to_send.trim().is_empty() {
            Message::send(app);
        }
        ui.add_space(4.0);
    }
}
