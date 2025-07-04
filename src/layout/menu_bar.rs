use crate::app::ChatApp;
use eframe::egui;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum NavigationItems {
    #[default]
    Rooms,
    Contacts,
}

pub struct MenuBar {}

impl MenuBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut app.context_menu, NavigationItems::Rooms, "Rooms");
            ui.selectable_value(&mut app.context_menu, NavigationItems::Contacts, "Contacts");
        });
        ui.add_space(10.0);
    }
}
