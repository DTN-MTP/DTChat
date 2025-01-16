use crate::app::ChatApp;
use crate::layout::informations::HeaderLayout;
use crate::layout::message_list_view::MessageListView;
use crate::layout::{message_forge::MessageForge, message_prompt::MessagePrompt};

use eframe::egui;

pub fn display(app: &mut ChatApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("header").show(ctx, |ui| {
        let mut header = HeaderLayout::new(app);
        header.show(app, ui);
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let mut body = MessageListView::new();
        body.show(app, ui);
    });

    egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
        let mut forge = MessageForge::new();
        forge.show(app, ui);
        ui.separator();
        let mut prompt = MessagePrompt::new();
        prompt.show(app, ui);
    });
}
