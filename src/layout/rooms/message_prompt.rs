use std::sync::{Arc, Mutex};

use crate::app::{AppEvent, ChatApp, ChatModel, MessageDirection};
use crate::utils::colors::COLORS;
use crate::utils::config::Peer;
use crate::utils::message::{ChatMessage, MessageStatus};
use crate::utils::proto::generate_uuid;
use crate::utils::socket::{GenericSocket, SendingSocket, TOKIO_RUNTIME};
use chrono::Utc;
use eframe::egui;
use egui::{vec2, CornerRadius, TextEdit};

pub struct MessagePrompt {}

pub fn manage_send(model: Arc<Mutex<ChatModel>>, text: &str, receiver: Peer) {
    let msg = ChatMessage {
        uuid: generate_uuid(),
        response: None,
        sender: model.lock().unwrap().localpeer.clone(),
        text: text.to_string(),
        shipment_status: MessageStatus::Sent(Utc::now()),
    };

    let socket = GenericSocket::new(&receiver.endpoints[0]);

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
                TOKIO_RUNTIME.spawn_blocking(move || {
                    manage_send(model_clone, &message_text, receiver_clone);
                });

                app.message_panel.message_to_send.clear();
            }
        }
        ui.add_space(4.0);
    }
}
