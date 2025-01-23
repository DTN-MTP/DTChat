mod app;
mod layout;
mod utils;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "DTCHat",
        options,
        Box::new(|_cc| {
            let mut app = app::ChatApp::default();
            if let Err(e) = app.try_connect_socket() {
                eprintln!("Socket connection failed: {}", e);
            }
            Ok(Box::new(app))
        }),
    )
}