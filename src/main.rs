use std::sync::{Arc, Mutex};
mod app;
mod layout;
mod utils;

use app::{ChatApp, ChatModel, EventHandler};
use utils::{
    config::AppConfigManager,
    socket::{DefaultSocketController, SocketController, SocketObserver},
};

#[derive(Clone)]
pub struct ArcChatApp {
    pub shared_app: Arc<Mutex<ChatApp>>,
}

fn main() -> Result<(), eframe::Error> {
    let config = AppConfigManager::load_yaml_from_file("database.yaml");

    let shared_peers = config.peer_list;
    let shared_rooms = config.room_list;
    let local_peer = config.local_peer;
    let model = ChatModel::new(
        shared_peers.clone(),
        local_peer.clone(),
        shared_rooms.clone(),
    );
    let model_arc = Arc::new(Mutex::new(model));

    let socket_controller = DefaultSocketController::init_controller(local_peer.clone()).unwrap();
    {
        socket_controller
            .lock()
            .unwrap()
            .add_observer(model_arc.clone() as Arc<dyn SocketObserver + Send + Sync>);
    }

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "DTCHat",
        options,
        Box::new(
            move |cc| -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
                let handler_arc = Arc::new(Mutex::new(EventHandler::new(cc.egui_ctx.clone())));
                model_arc.lock().unwrap().add_observer(handler_arc.clone());
                Ok(Box::new(ChatApp::new(model_arc, handler_arc)))
            },
        ),
    )?;
    Ok(())
}
