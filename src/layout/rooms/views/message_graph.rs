use crate::app::ChatApp;

pub struct MessageGraphView {}

impl MessageGraphView {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {}
}
