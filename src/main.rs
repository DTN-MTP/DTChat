mod app;
mod message;
mod ui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Chat App",
        options,
        Box::new(|_cc| Ok(Box::new(app::ChatApp::default()))),
    )
}
