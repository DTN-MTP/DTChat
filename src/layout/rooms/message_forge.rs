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
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Sender:");
            let forging_sender = app.message_panel.forging_sender.lock().unwrap();
            let current_sender_name = forging_sender.name.clone();
            drop(forging_sender);

            ComboBox::from_id_salt("Peer")
                .selected_text(current_sender_name)
                .show_ui(ui, |ui| {
                    for peer_arc in &app.peers {
                        let peer = peer_arc.lock().unwrap();
                        let is_selected = Arc::ptr_eq(&app.message_panel.forging_sender, peer_arc);
                        let display_name = peer.name.clone();
                        drop(peer);

                        if ui.selectable_label(is_selected, display_name).clicked() {
                            app.message_panel.forging_sender = Arc::clone(peer_arc);
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
