use crate::app::ChatApp;
use crate::message::SendByUser;
use eframe::egui;

pub struct HeaderLayout {
    // pub peers: Vec<String>,
    pub peer_endpoint: String,
}

impl HeaderLayout {
    pub fn new(app: &ChatApp) -> Self {
        // let peer_config = PeerConfig::load_from_file("peer-config.yaml");
        // let peers = peer_config
        //     .peer_list
        //     .iter()
        //     .map(|peer| peer.endpoint.clone())
        //     .collect();
        Self {
            peer_endpoint: app.peer_endpoint.clone(),
            // peers,
        }
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Context:");
            ui.radio_value(&mut app.ctx_sender_app, SendByUser::Earth, "Earth");
            ui.radio_value(&mut app.ctx_sender_app, SendByUser::March, "March");
        });
        ui.add_space(4.0);

        egui::Grid::new("header_grid")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label("Peer Endpoint:");
                ui.add_enabled(
                    false,
                    egui::TextEdit::singleline(&mut self.peer_endpoint.clone()),
                );
            });
        // ui.separator();
        // ui.add_space(10.0);
        // Peer List
        // ui.horizontal(|ui| {
        //     ui.label("Peer List:");
        //     egui::ComboBox::from_id_salt("Peers")
        //         .selected_text(format!("{:?}", self.peer_endpoint))
        //         .show_ui(ui, |ui| {
        //             for peer in &self.peers {
        //                 ui.selectable_value(&mut self.peer_endpoint, peer.clone(), peer);
        //             }
        //         });
        // });
        ui.add_space(10.0);
    }
}
