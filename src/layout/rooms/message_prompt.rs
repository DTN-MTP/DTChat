use std::sync::{Arc, Mutex};

use crate::app::{AppEvent, ChatApp, ChatModel, MessageDirection};
use crate::utils::colors::COLORS;
use crate::utils::config::Peer;
use crate::utils::message::{ChatMessage, MessageStatus};
use crate::utils::proto::generate_uuid;
use crate::network::Endpoint;
use crate::network::socket::TOKIO_RUNTIME;
use chrono::{DateTime, Utc};
use eframe::egui;
use egui::{vec2, CornerRadius, TextEdit};

// Parse the whole adress to ion_id
fn extract_ion_id_from_bp_address(bp_address: &str) -> String {
    if let Some(after_ipn) = bp_address.strip_prefix("ipn:") {
        if let Some(dot_pos) = after_ipn.find('.') {
            return after_ipn[..dot_pos].to_string();
        }
    }
    bp_address.to_string()
}

pub fn f64_to_utc(timestamp: f64) -> DateTime<Utc> {
    let secs = timestamp.trunc() as i64;
    let nsecs = ((timestamp.fract()) * 1_000_000_000.0).round() as u32;
    let naive = DateTime::from_timestamp(secs, nsecs).expect("Invalid timestamp");
    DateTime::from_naive_utc_and_offset(naive.naive_utc(), Utc)
}

pub struct MessagePrompt {}

pub fn manage_send(model: Arc<Mutex<ChatModel>>, msg: ChatMessage, receiver: Peer) {

    // Get the NetworkEngine from the model
    let network_engine = {
        let model_guard = model.lock().unwrap();
        match model_guard.get_network_engine() {
            Some(engine) => engine,
            None => {
                model_guard.notify_observers(AppEvent::Error("Network engine not available".to_string()));
                return;
            }
        }
    };

    // Clone things to move into async task
    let model_clone = Arc::clone(&model);
    let msg_clone = msg.clone();
    let receiver_uuid = receiver.uuid.clone();

    {
        model_clone
            .lock()
            .unwrap()
            .add_message(msg.clone(), MessageDirection::Sent);
    }

    // Spawn blocking task for network operations
    TOKIO_RUNTIME.spawn_blocking(move || {
        // Use NetworkEngine to send the message
        let engine = network_engine.lock().unwrap();
        if let Err(e) = engine.send_message_to_peer(&msg_clone, &receiver_uuid) {
            // On error, notify observers
            model_clone
                .lock()
                .unwrap()
                .notify_observers(AppEvent::Error(format!("Network error: {}", e)));
        }
    });
}

impl MessagePrompt {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        app.handler_arc
            .lock()
            .unwrap()
            .events
            .retain(|event| match event {
                AppEvent::Error(msg) | AppEvent::Sent(msg) | AppEvent::Received(msg) => {
                    app.message_panel.send_status = Some(msg.clone());
                    false
                }
            });
        ui.add_space(4.0);
        let mut send_message = false;
        ui.horizontal(|ui| {
            let text_edit = TextEdit::singleline(&mut app.message_panel.message_to_send)
                .hint_text("Write a message...")
                .desired_width(ui.available_width() - 200.0);
            let response = ui.add(text_edit);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                send_message = true;
                response.request_focus();
            }

            ui.checkbox(&mut app.message_panel.pbat_enabled, "PBAT");

            if ui
                .add(
                    egui::Button::new("Send")
                        .fill(COLORS[2])
                        .corner_radius(CornerRadius::same(2))
                        .min_size(vec2(65.0, 10.0)),
                )
                .clicked()
            {
                send_message = true;
            }
        });
        if send_message && !app.message_panel.message_to_send.trim().is_empty() {
            let forging_receiver = app.message_panel.forging_receiver.clone();
            if forging_receiver.name == "local peer" {
                app.message_panel.send_status =
                    Some("Cannot send message to local peer".to_string());
            } else {
                let model_clone = app.model_arc.clone();
                let receiver_clone = forging_receiver.clone();

                let mut prediction_time: Option<DateTime<Utc>> = None;

                // Do the prediction here
                if app.message_panel.pbat_enabled {
                    let sender_ion_id = {
                        let model_lock = app.model_arc.lock().unwrap();
                        let sender = &model_lock.localpeer;
                        let mut found_ion_id = None;

                        // Find BP endpoint in sender's endpoints
                        for endpoint in &sender.endpoints {
                            if let Endpoint::Bp(bp_address) = endpoint {
                                found_ion_id = Some(extract_ion_id_from_bp_address(bp_address));
                                break;
                            }
                        }

                        // Use found ION ID or fallback to UUID
                        found_ion_id.unwrap_or_else(|| sender.uuid.clone())
                    };

                    let receiver_ion_id =
                        if let Endpoint::Bp(bp_address) = &forging_receiver.endpoints[0] {
                            extract_ion_id_from_bp_address(bp_address)
                        } else {
                            forging_receiver.uuid.clone()
                        };
                    let message_size = app.message_panel.message_to_send.len() as f64;

                    let model_lock = app.model_arc.lock().unwrap();
                    if let Some(config) = &model_lock.prediction_config {
                        if let Ok(arrival_time) =
                            config.predict(&sender_ion_id, &receiver_ion_id, message_size)
                        {
                            prediction_time = Some(f64_to_utc(arrival_time));
                        }
                    }
                }

                let msg = ChatMessage {
                    uuid: generate_uuid(),
                    response: None,
                    sender: model_clone.lock().unwrap().localpeer.clone(),
                    text: app.message_panel.message_to_send.clone(),
                    shipment_status: MessageStatus::Sent(Utc::now(), prediction_time),
                };
                
                // Use TOKIO_RUNTIME.spawn_blocking for the synchronous operation
                TOKIO_RUNTIME.spawn_blocking(move || {
                    manage_send(model_clone, msg, receiver_clone);
                });

                app.message_panel.message_to_send.clear();
            }
        }
        ui.add_space(4.0);
    }
}
