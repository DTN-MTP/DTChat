mod app;
mod layout;
mod utils;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "DTCHat",
        options,
        Box::new(|_cc| Ok(Box::new(app::ChatApp::default()))),
    )
}
