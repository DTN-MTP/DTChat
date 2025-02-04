use std::sync::{Arc, Mutex};
mod app;
mod layout;
mod utils;

use app::{init_app, ChatApp};

#[derive(Clone)]
pub struct ArcChatApp {
    pub shared_app: Arc<Mutex<ChatApp>>,
}

impl eframe::App for ArcChatApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        let mut app = self.shared_app.lock().unwrap();
        app.update(ctx, frame);
    }
}

fn main() -> Result<(), eframe::Error> {
    let shared_app = init_app();
    let wrapper = ArcChatApp { shared_app };

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "DTCHat",
        options,
        Box::new(
            move |_cc| -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
                Ok(Box::new(wrapper.clone()))
            },
        ),
    )?;
    Ok(())
}
