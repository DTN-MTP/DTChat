use crate::app::ChatApp;
use crate::message::ContextView;
use eframe::egui;
use egui::TextEdit;

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
            ui.label("Context view:");
            egui::ComboBox::from_id_salt("context_view")
                .selected_text(format!("{:?}", app.ctx_view))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.ctx_view, ContextView::Me, "Me");

                    ui.selectable_value(&mut app.ctx_view, ContextView::Peer, "Peer")
                });
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Peer Endpoint:");
            ui.add_enabled(false, TextEdit::singleline(&mut self.peer_endpoint.clone()));
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
