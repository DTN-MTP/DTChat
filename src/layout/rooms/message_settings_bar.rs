use crate::app::ChatApp;
use eframe::egui;
use egui::{Align, ComboBox, Layout};
use std::sync::Arc;

use super::actions::create_room::CreateRoomForm;

#[derive(Debug, Clone, PartialEq)]
pub enum RoomView {
    Table,
    Graph,
    List,
}

impl Default for RoomView {
    fn default() -> Self {
        Self::List
    }
}

pub struct MessageSettingsBar {}

impl MessageSettingsBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let default_room_selected = app.message_panel.rooms[0].lock().unwrap().name.clone();

                ui.label("View:");
                ComboBox::from_id_salt("message_view")
                    .selected_text(format!("{:?}", app.message_panel.message_view))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.message_panel.message_view,
                            RoomView::Table,
                            "Table",
                        );
                        ui.selectable_value(
                            &mut app.message_panel.message_view,
                            RoomView::Graph,
                            "Graph",
                        );
                        ui.selectable_value(
                            &mut app.message_panel.message_view,
                            RoomView::List,
                            "List",
                        );
                    });

                ui.label("Room:");
                ComboBox::from_id_salt("room_list")
                    .selected_text(default_room_selected)
                    .show_ui(ui, |ui| {
                        for (i, room_arc) in app.message_panel.rooms.iter().enumerate() {
                            let room_name = room_arc.lock().unwrap().name.clone();
                            let is_selected = Arc::ptr_eq(&app.message_panel.rooms[0], room_arc);
                            if ui.selectable_label(is_selected, room_name).clicked() {
                                // app.message_panel.rooms.swap(0, i);
                            }
                        }
                    });
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("New Room").clicked() {
                    app.message_panel.create_modal_open = true;
                }
            });
        });

        if app.message_panel.create_modal_open {
            let mut create_room_modal = CreateRoomForm::new();
            create_room_modal.show(app, ui);
        }

        ui.add_space(4.0);
    }
}
