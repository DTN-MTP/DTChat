use chrono::{DateTime, Duration, Local, TimeZone};
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Chat App",
        options,
        Box::new(|_cc| Ok(Box::new(ChatApp::default()))),
    )
}

struct Message {
    send_time: String,
    receive_time: String,
    time_anchor: String,

    sent_by_me: bool,
    text: String,
}

struct ChatApp {
    messages: Vec<Message>, // List of messages
    input_text: String,     // Current input text
    sent_by_user: bool,     // True if "me", False if "someone else"
    send_time: String,      // Manual sending time input
    receive_time: String,   // Manual receiving time input
}

impl Default for ChatApp {
    fn default() -> Self {
        let recv_time = Local::now() + Duration::hours(1);
        Self {
            messages: Vec::new(),
            input_text: String::new(),
            sent_by_user: true, // Default to "me"
            send_time: Local::now().format("%H:%M:%S").to_string(),
            receive_time: recv_time.format("%H:%M:%S").to_string(),
        }
    }
}

impl eframe::App for ChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::TopBottomPanel::bottom("input_panel")
                .resizable(false)
                .min_height(0.0)
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Send Time:");
                        ui.text_edit_singleline(&mut self.send_time);
                        ui.label("Receive Time:");
                        ui.text_edit_singleline(&mut self.receive_time);
                    });

                    ui.add_space(4.0);

                    let mut send_message = false;
                    ui.horizontal(|ui| {
                        let text_edit = egui::TextEdit::singleline(&mut self.input_text)
                            .hint_text("Write a message...")
                            .desired_width(ui.available_width() - 200.0);

                        let response = ui.add(text_edit);
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            send_message = true;
                        }

                        if self.sent_by_user {
                            ui.colored_label(egui::Color32::GREEN, "Me");
                        } else {
                            ui.colored_label(egui::Color32::WHITE, "Me");
                        }
                        ui.radio_value(&mut self.sent_by_user, true, "");

                        if !self.sent_by_user {
                            ui.colored_label(egui::Color32::RED, "Other");
                        } else {
                            ui.colored_label(egui::Color32::WHITE, "Other");
                        }
                        ui.radio_value(&mut self.sent_by_user, false, "");

                        if ui.button("Send").clicked() {
                            send_message = true;
                        }
                    });

                    if send_message && !self.input_text.trim().is_empty() {
                        let anchor = if self.sent_by_user {
                            self.receive_time.clone()
                        } else {
                            self.send_time.clone()
                        };
                        self.messages.push(Message {
                            send_time: self.send_time.clone(),
                            receive_time: self.receive_time.clone(),
                            time_anchor: anchor,
                            sent_by_me: self.sent_by_user,
                            text: self.input_text.clone(),
                        });
                        self.messages
                            .sort_by(|a, b| a.time_anchor.cmp(&b.time_anchor));
                        self.input_text.clear();
                    }
                });

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for message in &self.messages {
                        let text_color = if message.sent_by_me {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::RED
                        };
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "[{}/{}] {}",
                                    message.send_time, message.receive_time, message.text
                                ))
                                .color(text_color),
                            );
                        });
                    }
                });
        });
    }
}
