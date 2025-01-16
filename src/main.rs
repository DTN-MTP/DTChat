mod app;
mod layout;
mod message;
mod peer_config;
mod ui;
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "DTCHat",
        options,
        Box::new(|_cc| Ok(Box::new(app::ChatApp::default()))),
    )
}
