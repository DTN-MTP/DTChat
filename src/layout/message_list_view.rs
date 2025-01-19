use crate::app::ChatApp;
// Removed: use crate::peer_config::SharedPeer; 
use egui::ComboBox;
use std::rc::Rc;

pub struct MessageListView {}

impl MessageListView {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        let mut call_sort = false;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("See view from:");

                    let current_view_from = app.show_view_from.borrow().name.clone();

                    ComboBox::from_id_salt("Peer")
                        .selected_text(current_view_from)
                        .show_ui(ui, |ui| {
                            for peer_rc in &app.peers {
                                let peer = peer_rc.borrow();
                                let peer_name = peer.name.clone();
                                let is_selected = Rc::ptr_eq(&app.show_view_from, peer_rc);

                                // Use selectable_label and manually handle selection
                                if ui.selectable_label(is_selected, peer_name.clone()).clicked() {
                                    app.show_view_from = Rc::clone(peer_rc);
                                    call_sort = true;
                                }
                            }
                        });
                });

                if call_sort {
                    app.sort_messages();
                }

                ui.separator();

                for message in &app.messages {
                    ui.horizontal(|ui| {
                        let sender_color = message.sender.borrow().get_color();

                        ui.label(
                            egui::RichText::new(format!(
                                "{}: {}",
                                message.get_shipment_status_str(),
                                message.text
                            ))
                            .color(sender_color),
                        );
                    });
                }
            });
    }
}
