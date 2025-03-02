use crate::app::{ChatApp, ChatModel, SortStrategy};
use egui::ComboBox;
use std::sync::{Arc, Mutex};

pub struct MessageListView {}

impl MessageListView {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        app.handler_arc
            .lock()
            .unwrap()
            .events
            .retain(|event| match event {
                crate::app::AppEvent::MessageReceived(_message) => {
                    app.message_panel.send_status = Some("Message received".to_string());
                    false
                }
                _ => true,
            });

        let mut locked_model = app.model_arc.lock().unwrap();
        let sort_for_peer = locked_model.localpeer.clone();
        let mut call_sort = false;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("See view from:");

                    ComboBox::from_id_salt("Peer")
                        .selected_text(sort_for_peer.name.clone())
                        .show_ui(ui, |ui| {
                            for peer in &locked_model.peers {
                                if ui
                                    .selectable_label(
                                        peer.uuid == sort_for_peer.uuid,
                                        peer.name.clone(),
                                    )
                                    .clicked()
                                {
                                    call_sort = true;
                                }
                            }
                        });
                });
                if call_sort {
                    locked_model.sort_messages(SortStrategy::Relative(sort_for_peer.uuid));
                }

                for message in &locked_model.messages {
                    ui.horizontal(|ui| {
                        let color = message.sender.get_color();
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
