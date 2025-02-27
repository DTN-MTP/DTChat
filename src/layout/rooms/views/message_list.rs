use crate::app::ChatApp;
use egui::ComboBox;
use std::sync::Arc;

pub struct MessageListView {}

impl MessageListView {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        let mut call_sort = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("See view from:");
                    let show_view = app.message_panel.show_view_from.lock().unwrap();
                    let current_view_from = show_view.name.clone();
                    drop(show_view);

                    ComboBox::from_id_salt("Peer")
                        .selected_text(current_view_from)
                        .show_ui(ui, |ui| {
                            for peer_arc in &app.peers {
                                let peer_lock = peer_arc.lock().unwrap();
                                let is_selected =
                                    Arc::ptr_eq(&app.message_panel.show_view_from, peer_arc);
                                let peer_name = peer_lock.name.clone();
                                drop(peer_lock);

                                if ui.selectable_label(is_selected, peer_name).clicked() {
                                    app.message_panel.show_view_from = Arc::clone(peer_arc);
                                    call_sort = Some(peer_arc.lock().as_ref().unwrap().name.clone());
                                }
                            }
                        });
                });

                if let Some(peer_name) = call_sort {
                    app.sort_messages(peer_name);
                }

                for message in &app.model.lock().unwrap().messages {
                    ui.horizontal(|ui| {
                        let sender_lock = message.sender.lock().unwrap();
                        let color = sender_lock.get_color();
                        drop(sender_lock);

                        ui.label(
                            egui::RichText::new(format!(
                                "{}: {}",
                                message.get_shipment_status_str(),
                                message.text
                            ))
                            .color(color),
                        );
                    });
                }
            });
    }
}
