use super::{
    components::{message_forge::MessageForge, message_input::MessagePrompt},
    menu::{NavigationItems, Header},
    views::{
        message_graph::MessageGraphView,
        message_list::MessageListView,
        message_settings_bar::{MessageSettingsBar, RoomView},
    },
};
use crate::app::ChatApp;
use eframe::egui;
use egui::{CentralPanel, TopBottomPanel};

pub fn display(app: &mut ChatApp, ctx: &egui::Context) {
    // Corporate header at the top
    TopBottomPanel::top("corporate_header").show(ctx, |ui| {
        let mut header = Header::new();
        header.show(ui);
    });

    // TODO: Uncomment this when we have a menu bar ready.
    // TopBottomPanel::top("menu_bar").show(ctx, |ui| {
    //     let mut menu = MenuBar::new();
    //     menu.show(app, ui);
    // });

    match app.context_menu {
        NavigationItems::Rooms => {
            TopBottomPanel::top("message_settings_bar").show(ctx, |ui| {
                MessageSettingsBar::new().show(app, ui);
            });

            TopBottomPanel::bottom("message_inputs_panel").show(ctx, |ui| {
                let mut forge = MessageForge::new();
                forge.show(app, ui);
                ui.separator();
                let mut prompt = MessagePrompt::new();
                prompt.show(app, ui);
                ui.separator();
                if let Some(status) = &app.message_panel.send_status {
                    ui.label(status);
                } else {
                    ui.label("");
                }
            });

            CentralPanel::default().show(ctx, |ui| match app.message_panel.message_view {
                RoomView::Table => {
                    ui.label("Table View");
                }
                RoomView::Graph => {
                    let mut message_graph = MessageGraphView::new();
                    message_graph.show(app, ui);
                }
                RoomView::List => {
                    let mut message_list = MessageListView::new();
                    message_list.show(app, ui);
                }
            });
        }
        NavigationItems::Contacts => {
            CentralPanel::default().show(ctx, |ui| {
                ui.label("Contacts");
            });
        }
    }
}
