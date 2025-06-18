use std::sync::{Arc, Mutex};
use std::path::Path;
mod app;
mod layout;
mod utils;

use app::{ChatApp, ChatModel, EventHandler};
use chrono::{Duration, Utc};
use utils::{
    config::AppConfigManager,
    message::{ChatMessage, MessageStatus},
    proto::generate_uuid,
    socket::{DefaultSocketController, SocketController, SocketObserver},
    network_config::NetworkConfig,
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
    let contact_plan = config.a_sabr;

    if !Path::new(&contact_plan).exists(){
        eprintln!("Contact plan missing !!!");
    }

    let mut now = Utc::now() - Duration::seconds(40);

    let network_config = match NetworkConfig::new(&contact_plan) {
        Ok(config) => Some(config),
        Err(e) => {
            eprintln!("Failed to create NetworkConfig: {}", e);
            None
        }
    };

    let mut model = ChatModel::new(
        shared_peers.clone(),
        local_peer.clone(),
        shared_rooms.clone(),
        network_config
    );

    #[cfg(feature = "dev")]
    {
        model.messages.push(ChatMessage {
            uuid: generate_uuid(),
            response: None,
            sender: local_peer.clone(),
            text: "Hello from local peer".to_owned(),
            shipment_status: MessageStatus::Received(now, now + Duration::seconds(10)),
        });

        now += Duration::seconds(2);

        model.messages.push(ChatMessage {
            uuid: generate_uuid(),
            response: None,
            sender: shared_peers[2].clone(),
            text: "Bob at your service !".to_owned(),
            shipment_status: MessageStatus::Received(now, now + Duration::seconds(30)),
        });

        now += Duration::seconds(1);

        model.messages.push(ChatMessage {
            uuid: generate_uuid(),
            response: None,
            sender: shared_peers[0].clone(),
            text: "Hello local peer, how are you?".to_owned(),
            shipment_status: MessageStatus::Received(now, now + Duration::seconds(10)),
        });

        now += Duration::seconds(2);

        model.messages.push(ChatMessage {
            uuid: generate_uuid(),
            response: None,
            sender: shared_peers[0].clone(),
            text: "I'm john does".to_owned(),
            shipment_status: MessageStatus::Received(now, now + Duration::seconds(10)),
        });

        now += Duration::seconds(13);

        model.messages.push(ChatMessage {
            uuid: generate_uuid(),
            response: None,
            sender: local_peer.clone(),
            text: "Hello john doe, Some news from alice ?".to_owned(),
            shipment_status: MessageStatus::Received(now, now + Duration::seconds(10)),
        });

        now += Duration::seconds(5);

        model.messages.push(ChatMessage {
            uuid: generate_uuid(),
            response: None,
            sender: shared_peers[1].clone(),
            text: "Sorry, I'm a bit late!".to_owned(),
            shipment_status: MessageStatus::Received(now, now + Duration::seconds(12)),
        });
    }

    let model_arc = Arc::new(Mutex::new(model));

    match DefaultSocketController::init_controller(local_peer.clone(), shared_peers.clone()) {
        Ok(controller) => {
            controller
                .lock()
                .unwrap()
                .add_observer(model_arc.clone() as Arc<dyn SocketObserver + Send + Sync>);
        }
        Err(e) => {
            eprintln!("Failed to initialize socket controller: {:?}", e);
        }
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
