use crate::app::ChatApp;
use crate::message::Message;
use eframe::egui;

const COLOR_ME: egui::Color32 = egui::Color32::GREEN;
const COLOR_OTHER: egui::Color32 = egui::Color32::RED;

pub struct BodyLayout {
    pub messages: Vec<Message>,
}

impl BodyLayout {
    pub fn new(app: &ChatApp) -> Self {
        Self {
            messages: app.messages.clone(),
        }
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for message in &self.messages {
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
}
