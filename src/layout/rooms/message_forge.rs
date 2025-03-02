use crate::app::ChatApp;
use eframe::egui;
use egui::{ComboBox, TextEdit};
use std::sync::Arc;

pub struct MessageForge {}

impl MessageForge {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        let locked_model = app.model_arc.lock().unwrap();

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Send to:");
            let forging_receiver = app.message_panel.forging_receiver.clone();

            ComboBox::from_id_salt("Peer")
                .selected_text(forging_receiver.name.clone())
                .show_ui(ui, |ui| {
                    for peer in &locked_model.peers {
                        if ui
                            .selectable_label(forging_receiver.uuid == peer.uuid, peer.name.clone())
                            .clicked()
                        {
                            app.message_panel.forging_receiver = peer.clone();
                        }
                    }
                });

            ui.add_space(4.0);
            ui.label("Send Time:");
            ui.add(
                TextEdit::singleline(&mut app.message_panel.forging_tx_time).desired_width(100.0),
            );

            ui.label("Receive Time:");
            ui.add(
                TextEdit::singleline(&mut app.message_panel.forging_rx_time).desired_width(100.0),
            );
        });
        ui.add_space(4.0);
    }
}
