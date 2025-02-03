use crate::app::ChatApp;
use egui::ComboBox;
use std::rc::Rc;

pub struct MessageListView {}

impl MessageListView {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        app.sort_messages();
        let mut call_sort = false;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("See view from:");
                    let current_view_from = app.message_panel.show_view_from.borrow().name.clone();
                    ComboBox::from_id_salt("Peer")
                        .selected_text(current_view_from)
                        .show_ui(ui, |ui| {
                            for peer_rc in &app.peers {
                                let is_selected =
                                    Rc::ptr_eq(&app.message_panel.show_view_from, peer_rc);
                                if ui
                                    .selectable_label(is_selected, peer_rc.borrow().name.clone())
                                    .clicked()
                                {
                                    app.message_panel.show_view_from = Rc::clone(peer_rc);
                                    call_sort = true;
                                }
                            }
                        });
                });
                if call_sort {
                    app.sort_messages();
                }
                for message in &app.message_panel.messages {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}: {}",
                                message.get_shipment_status_str(),
                                message.text
                            ))
                            .color(message.sender.borrow().get_color()),
                        );
                    });
                }
            });
    }
}
