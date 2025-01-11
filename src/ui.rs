use crate::app::{ChatApp, ConnectionStatus};
use crate::message::Message;
use eframe::egui;

const COLOR_ME: egui::Color32 = egui::Color32::GREEN;
const COLOR_OTHER: egui::Color32 = egui::Color32::RED;
const COLOR_DEFAULT: egui::Color32 = egui::Color32::WHITE;

pub fn update_ui(app: &mut ChatApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("header").show(ctx, |ui| {
        show_connection_status(app, ui);
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        show_messages(app, ui);
    });

    egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
        show_input_panel(app, ui);
    });
}

pub fn get_status_color(status: &ConnectionStatus) -> egui::Color32 {
    match status {
        ConnectionStatus::Connected => egui::Color32::GREEN,
        ConnectionStatus::Disconnected => egui::Color32::RED,
    }
}

fn show_connection_status(app: &ChatApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        let status_text = match app.connection_status {
            ConnectionStatus::Connected => "Connected",
            ConnectionStatus::Disconnected => "Disconnected",
        };

        let status_color: egui::Color32 = get_status_color(&app.connection_status);
        ui.label("Status:");
        ui.colored_label(status_color, status_text);
    });

    ui.add_space(10.0);

    egui::Grid::new("connection_status_grid")
        .num_columns(2)
        .spacing([10.0, 4.0])
        .show(ui, |ui| {
            ui.label("Local Endpoint:");
            ui.add_enabled(
                false,
                egui::TextEdit::singleline(&mut app.local_endpoint.clone()),
            );
            ui.end_row();

            ui.label("Peer Endpoint:");
            ui.add_enabled(
                false,
                egui::TextEdit::singleline(&mut app.peer_endpoint.clone()),
            );
            ui.end_row();
        });

    ui.add_space(10.0);
}

fn show_input_panel(app: &mut ChatApp, ui: &mut egui::Ui) {
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label("Send Time:");
        ui.text_edit_singleline(&mut app.send_time);
        ui.label("Receive Time:");
        ui.text_edit_singleline(&mut app.receive_time);
    });

    ui.add_space(4.0);

    let mut send_message = false;
    ui.horizontal(|ui| {
        let text_edit = egui::TextEdit::singleline(&mut app.input_text)
            .hint_text("Write a message...")
            .desired_width(ui.available_width() - 200.0);

        let response = ui.add(text_edit);
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            send_message = true;
        }

        if app.sent_by_user {
            ui.colored_label(COLOR_ME, "Me");
        } else {
            ui.colored_label(COLOR_DEFAULT, "Me");
        }
        ui.radio_value(&mut app.sent_by_user, true, "");

        if !app.sent_by_user {
            ui.colored_label(COLOR_OTHER, "Other");
        } else {
            ui.colored_label(COLOR_DEFAULT, "Other");
        }
        ui.radio_value(&mut app.sent_by_user, false, "");

        if ui.button("Send").clicked() {
            send_message = true;
        }
    });

    if send_message && !app.input_text.trim().is_empty() {
        let anchor = if app.sent_by_user {
            app.receive_time.clone()
        } else {
            app.send_time.clone()
        };
        app.messages.push(Message {
            time_anchor: anchor,
            sent_by_me: app.sent_by_user,
            text: app.input_text.clone(),
        });
        app.messages
            .sort_by(|a, b| a.time_anchor.cmp(&b.time_anchor));
        app.input_text.clear();
    }
}

fn show_messages(app: &ChatApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            for message in &app.messages {
                let text_color = if message.sent_by_me {
                    COLOR_ME
                } else {
                    COLOR_OTHER
                };
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("[{}] {}", message.time_anchor, message.text))
                            .color(text_color),
                    );
                });
            }
        });
}
