use crate::app::ChatApp;
use eframe::egui;
use egui::{Id, Modal};

pub struct CreateRoomForm {}

impl CreateRoomForm {
    pub fn new() -> Self {
        Self {}
    }
    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        Modal::new(Id::new("create_room_modal")).show(ui.ctx(), |ui| {
            ui.set_width(250.0);

            ui.heading("Create a new room");

            ui.separator();

            if ui.button("Cancel").clicked() {
                app.message_panel.create_modal_open = false;
            }
        });
    }
}
