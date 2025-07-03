use crate::app::{ChatApp, SortStrategy};

pub struct MessageListView {}

fn get_str_for_strat(local_peer_uuid: &String, strat: &SortStrategy) -> String {
    match strat {
        SortStrategy::Standard => "Standard".to_string(),
        SortStrategy::Relative(peer) => {
            if peer.uuid == *local_peer_uuid {
                "Local".to_string()
            } else {
                format!("Relative ({})", peer.name)
            }
        }
    }
}

impl MessageListView {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        let mut locked_model = app.model_arc.lock().unwrap();
        //let sort_for_peer = locked_model.localpeer.clone();
        let sort_strat = locked_model.sort_strategy.clone();
        let local_peer = locked_model.localpeer.clone();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Message sorting strategy:");

                    ui.menu_button(get_str_for_strat(&local_peer.uuid, &sort_strat), |ui| {
                        if ui.button("Standard").on_hover_text("Sorted by sending times").clicked() {
                            locked_model.sort_messages(SortStrategy::Standard);
                            ui.close_menu();
                        }
                        if ui.button("Local").on_hover_text("Sorted by receiving time for the local peer and sending times for the other peers").clicked() {
                            locked_model.sort_messages(SortStrategy::Relative(local_peer.clone()));
                            ui.close_menu();
                        }
                        ui.menu_button("Relative", |ui| {
                            let mut clicked = None;

                            for peer in &locked_model.peers {
                                if ui.button(peer.name.as_str()).on_hover_text(format!("Sorted by receiving time for peer {} and sending times for the other peers", peer.name)).clicked() {
                                    clicked = Some(peer.clone());
                                }
                             }
                             if let Some(peer) = clicked {
                                locked_model.sort_messages(SortStrategy::Relative(peer.clone()));
                                ui.close_menu();
                             }

                        });

                    });
                });

                for message in &locked_model.messages {
                    ui.horizontal(|ui| {
                        let color = message.sender.get_color();
                        let sent_by_me = local_peer.uuid == message.sender.uuid;
                        // Add visual indicator for ACK status
                        let ack_indicator = match &message.shipment_status {
                            crate::domain::MessageStatus::Sent(_, _) if sent_by_me => "⏳", // Waiting for ACK
                            crate::domain::MessageStatus::Received(_, _) if sent_by_me => "✅", // ACK received
                            crate::domain::MessageStatus::Received(_, _) => "📨", // Received message
                            _ => "📤", // Sent by others
                        };
                        ui.label(egui::RichText::new(ack_indicator).size(16.0));
                        ui.label(
                            egui::RichText::new(format!(
                                "{}: {}",
                                message.get_shipment_status_str(sent_by_me),
                                message.text
                            ))
                            .color(color),
                        );
                    });
                }
            });
    }
}
