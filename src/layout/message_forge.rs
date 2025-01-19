use crate::app::ChatApp;
use eframe::egui;
use egui::{ComboBox, TextEdit};
use std::rc::Rc;

pub struct MessageForge {}

impl MessageForge {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Sender:");

            let current_sender_name = app.forging_sender.borrow().name.clone();

            ComboBox::from_id_salt("Peer")
                .selected_text(current_sender_name)
                .show_ui(ui, |ui| {
                    for peer_rc in &app.peers {
                         let is_selected = Rc::ptr_eq(&app.forging_sender, peer_rc);
                        // Use selectable_label and manually handle selection
                        if ui.selectable_label(is_selected, peer_rc.borrow().name.clone()).clicked() {
                            app.forging_sender = Rc::clone(peer_rc);
                        }
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
