use crate::app::ChatApp;

use eframe::egui;
use egui::{ComboBox, TextEdit};

pub struct MessageForge {}

impl MessageForge {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Sender:");
            ComboBox::from_id_salt("Peer")
                .selected_text(format!("{:?}", app.forging_sender.name))
                .show_ui(ui, |ui| {
                    for peer in &app.peers {
                        ui.selectable_value(
                            &mut app.forging_sender,
                            peer.clone(),
                            peer.name.clone(),
                        );
                    }
                });
            ui.add_space(4.0);
            ui.label("Send Time:");
            ui.add(TextEdit::singleline(&mut app.forging_tx_time).desired_width(100.0));
            ui.label("Receive Time:");
            ui.add(TextEdit::singleline(&mut app.forging_rx_time).desired_width(100.0));
        });
        ui.add_space(4.0);
    }
}
