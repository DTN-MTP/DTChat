use crate::app::ChatApp;
use crate::layout::body::BodyLayout;
use crate::layout::footer::FooterLayout;
use crate::layout::header::HeaderLayout;

use chrono::Local;
use eframe::egui;

pub fn display(app: &mut ChatApp, ctx: &egui::Context) {
    app.send_time = Local::now().format("%H:%M:%S").to_string();

    egui::TopBottomPanel::top("header").show(ctx, |ui| {
        let header = HeaderLayout::new(app);
        header.show(ui);
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let body = BodyLayout::new(app);
        body.show(ui);
    });

    egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
        let footer = FooterLayout::new(app);
        footer.show(app, ui);
    });
}
