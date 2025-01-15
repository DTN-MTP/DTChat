use crate::app::ChatApp;
use crate::layout::body::BodyLayout;
use crate::layout::footer::FooterLayout;
use crate::layout::header::HeaderLayout;

use eframe::egui;

pub fn display(app: &mut ChatApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("header").show(ctx, |ui| {
        let mut header = HeaderLayout::new(app);
        header.show(app, ui);
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let body = BodyLayout::new(app);
        body.show(ui);
    });

    egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
        let mut footer = FooterLayout::new();
        footer.show(app, ui);
    });
}
