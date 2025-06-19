use std::sync::{Arc, Mutex};

use crate::app::{AppEvent, ChatApp, ChatModel, MessageDirection};
use crate::utils::colors::COLORS;
use crate::utils::config::Peer;
use crate::utils::message::{ChatMessage, MessageStatus};
use crate::utils::network_config::NetworkConfig;
use crate::utils::proto::generate_uuid;
use crate::utils::socket::{Endpoint, GenericSocket, SendingSocket, TOKIO_RUNTIME};
use chrono::Utc;
use eframe::egui;
use egui::{vec2, CornerRadius, TextEdit};
use libc::UTIME_NOW;


// Parse the whole adress to ion_id
fn extract_ion_id_from_bp_address(bp_address: &str) -> String {
    if bp_address.starts_with("ipn:") {
        let after_ipn = &bp_address[4..];
        if let Some(dot_pos) = after_ipn.find('.') {
            return after_ipn[..dot_pos].to_string();
        }
    }
    bp_address.to_string()
}


pub struct MessagePrompt {}

// ...existing code...

pub fn manage_send(model: Arc<Mutex<ChatModel>>, msg: ChatMessage, receiver: Peer) {
    // Check if network config exists and test endpoint
    let endpoint_test_result = {
        let model_lock = model.lock().unwrap();
        if let Some(config) = &model_lock.network_config {
            config.test_endpoint(&receiver.endpoints[0])
        } else {
            false
        }
    };

    if !endpoint_test_result {
        model
            .lock()
            .unwrap()
            .notify_observers(AppEvent::MessageError(
                "Contact absent from the contact plan".to_string(),
            ));
        return;
    }

    if msg.pbat_enabled {
        if let Endpoint::Bp(_) = &receiver.endpoints[0] {
            let sender_ion_id = {
                let mut found_ion_id = None;
                // Find BP endpoint in sender's endpoints
                for endpoint in &msg.sender.endpoints {
                    if let Endpoint::Bp(bp_address) = endpoint {
                        found_ion_id = Some(extract_ion_id_from_bp_address(bp_address));
                        break;
                    }
                }
                // Use found ION ID or fallback to UUID
                found_ion_id.unwrap_or_else(|| msg.sender.uuid.clone())
            };
            let receiver_ion_id = if let Endpoint::Bp(bp_address) = &receiver.endpoints[0] {
                extract_ion_id_from_bp_address(bp_address)
            } else {
                receiver.uuid.clone()
            };

            let model_lock = model.lock().unwrap();
            if let Some(config) = &model_lock.network_config {
                let message_size = msg.text.len() as f64;
                match config.route_with_ion_ids(&sender_ion_id, &receiver_ion_id, message_size) {
                    Ok(time_value) => {
                        println!("✅ the PBAT is {}", time_value);
                    }
                    Err(e) => {
                        eprintln!("⚠️ Routing error: {}", e);
                    }
                }
            }
        }
    }

    let socket = GenericSocket::new(&receiver.endpoints[0]);
    println!("the receivers endpoint is : {:?}", receiver.endpoints[0]);

    match socket {
        Ok(mut socket) => match socket.send_message(&msg) {
            Ok(_) => {
                let mut model_locked = model.lock().unwrap();
                model_locked.add_message(msg.clone(), MessageDirection::Sent);
            }
            Err(_) => model
                .lock()
                .unwrap()
                .notify_observers(AppEvent::MessageError("Socket error.".to_string())),
        },
        Err(_) => model
            .lock()
            .unwrap()
            .notify_observers(AppEvent::MessageError(
                "Socket initialization failed.".to_string(),
            )),
    }
}

// ...existing code...

impl MessagePrompt {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        app.handler_arc
            .lock()
            .unwrap()
            .events
            .retain(|event| match event {
                AppEvent::MessageError(msg)
                | AppEvent::MessageSent(msg)
                | AppEvent::MessageReceived(msg) => {
                    app.message_panel.send_status = Some(msg.clone());
                    false
                }
                _ => true,
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
                let message_text = app.message_panel.message_to_send.clone();
                let model_clone = app.model_arc.clone();
                let receiver_clone = forging_receiver.clone();

                let msg = ChatMessage {
                    uuid: generate_uuid(),
                    response: None,
                    sender: model_clone.lock().unwrap().localpeer.clone(),
                    text: message_text.clone(),
                    shipment_status: MessageStatus::Sent(Utc::now()),
                    pbat_enabled : app.message_panel.pbat_enabled.clone()
                };
                TOKIO_RUNTIME.spawn_blocking(move || {
                    manage_send(model_clone, msg,receiver_clone);
                });

                app.message_panel.message_to_send.clear();
            }
        }
        ui.add_space(4.0);
    }
}
