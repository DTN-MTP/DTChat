use crate::app::ChatApp;
use eframe::egui;

pub struct HeaderLayout {
    pub local_endpoint: String,
    pub peer_endpoint: String,
    pub connection_status: ConnectionStatus,
}

#[derive(PartialEq, Clone)]
#[allow(dead_code)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

impl HeaderLayout {
    pub fn new(app: &ChatApp) -> Self {
        Self {
            local_endpoint: app.local_endpoint.clone(),
            peer_endpoint: app.peer_endpoint.clone(),
            connection_status: app.connection_status.clone(),
        }
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            let status_text = match self.connection_status {
                ConnectionStatus::Connected => "Connected",
                ConnectionStatus::Disconnected => "Disconnected",
            };

            let status_color: egui::Color32 = get_status_color(&self.connection_status);
            ui.label("Status:");
            ui.colored_label(status_color, status_text);
        });

        ui.add_space(10.0);

        egui::Grid::new("connection_status_grid")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label("Local Endpoint:");
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.local_endpoint.clone()),
                );
                ui.end_row();

                ui.label("Peer Endpoint:");
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.peer_endpoint.clone()),
                );
                ui.end_row();
            });

        ui.add_space(10.0);
    }
}

fn get_status_color(status: &ConnectionStatus) -> egui::Color32 {
    match status {
        ConnectionStatus::Connected => egui::Color32::GREEN,
        ConnectionStatus::Disconnected => egui::Color32::RED,
    }
}
