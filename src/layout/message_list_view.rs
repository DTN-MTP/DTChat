use crate::app::ChatApp;

use eframe::egui;
use egui::ComboBox;

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
                    ui.label("Sender:");
                    ComboBox::from_id_salt("Peer")
                        .selected_text(format!("{:?}", app.show_view_from.name))
                        .show_ui(ui, |ui| {
                            for peer in &app.peers {
                                let response = ui.selectable_value(
                                    &mut app.show_view_from,
                                    peer.clone(),
                                    peer.name.clone(),
                                );
                                if response.changed() {
                                    call_sort = true
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
                        ui.label(
                            egui::RichText::new(format!(
                                "{}: {}",
                                message.get_shipment_status_str(),
                                message.text
                            ))
                            .color(message.sender.get_color()),
                        );
                    });
                }
            });
    }
}
