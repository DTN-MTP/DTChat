use crate::app::ChatApp;
use crate::message::Message;
use eframe::egui;

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
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!(
                                "[{}] [{}] [{}] [{}]",
                                message.send_time,
                                message.get_shipment_status_str(),
                                message.get_type_str(),
                                message.get_priority_str()
                            )));
                        });

                        ui.label(egui::RichText::new(format!("{}", message.text)));
                    });

                    ui.separator();
                }
            });
    }
}
