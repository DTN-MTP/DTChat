use crate::app::ChatApp;

pub struct MessageTableView {}

impl MessageTableView {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {}
}
