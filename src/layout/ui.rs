use super::{
    menu_bar::NavigationItems,
    rooms::{
        message_settings_bar::{MessageSettingsBar, RoomView},
        views::{message_graph::MessageGraphView, message_list::MessageListView},
    },
};
use crate::app::ChatApp;
use crate::layout::menu_bar::MenuBar;
use crate::layout::rooms::message_forge::MessageForge;
use crate::layout::rooms::message_prompt::MessagePrompt;
use eframe::egui;
use egui::{CentralPanel, TopBottomPanel};

pub fn display(app: &mut ChatApp, ctx: &egui::Context) {
    TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        let mut menu = MenuBar::new();
        menu.show(app, ui);
    });

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
