use crate::app::{ChatApp, ViewToDisplay};
use crate::message::Message;
use eframe::egui;
use egui::ComboBox;

pub struct BodyLayout {
    pub messages: Vec<Message>,
}

impl BodyLayout {
    pub fn new(app: &ChatApp) -> Self {
        Self {
            messages: app.messages.clone(),
        }
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("View to Display:");
                    ComboBox::from_id_salt("view_to_display")
                        .selected_text(format!("{:?}", app.view_to_display))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.view_to_display,
                                ViewToDisplay::Table,
                                "Table",
                            );
                            ui.selectable_value(
                                &mut app.view_to_display,
                                ViewToDisplay::LinearGraph,
                                "Linear Graph",
                            );
                            ui.selectable_value(
                                &mut app.view_to_display,
                                ViewToDisplay::Tchat,
                                "Tchat",
                            );
                        });
                });

                for message in &self.messages {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!(
                                "[{}] [{}]",
                                message.send_time,
                                message.get_shipment_status_str(),
                            )));
                        });

                        ui.label(egui::RichText::new(format!("{}", message.text)));
                    });

                    ui.separator();
                }
            });
    }
}
