use eframe::egui;
use crate::app::ChatApp;
use crate::message::Message;

const COLOR_ME: egui::Color32 = egui::Color32::GREEN;
const COLOR_OTHER: egui::Color32 = egui::Color32::RED;
const COLOR_DEFAULT: egui::Color32 = egui::Color32::WHITE;

pub fn update_ui(app: &mut ChatApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        show_input_panel(app, ui);
        show_messages(app, ui);
    });
}

fn show_input_panel(app: &mut ChatApp, ui: &mut egui::Ui) {
    egui::TopBottomPanel::bottom("input_panel")
        .resizable(false)
        .min_height(0.0)
        .show_inside(ui, |ui| {
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
                    //send_time: app.send_time.clone(),
                    //receive_time: app.receive_time.clone(),
                    time_anchor: anchor,
                    sent_by_me: app.sent_by_user,
                    text: app.input_text.clone(),
                });
                app.messages.sort_by(|a, b| a.time_anchor.cmp(&b.time_anchor));
                app.input_text.clear();
            }
        });
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
                        egui::RichText::new(format!(
                            "[{}] {}",
                            message.time_anchor, message.text
                        ))
                        .color(text_color),
                    );
                });
            }
        });
}