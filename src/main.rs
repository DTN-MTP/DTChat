use std::path::Path;
use std::sync::{Arc, Mutex};
mod app;
mod layout;
mod network;
mod utils;

use app::{ChatApp, ChatModel, EventHandler};

#[cfg(feature = "dev")]
use chrono::{Duration, Utc};

use utils::{
    ack_config,
    config::AppConfigManager,
    prediction_config::PredictionConfig,
};

use network::{NetworkEngine};

#[cfg(feature = "dev")]
use utils::{
    message::{ChatMessage, MessageStatus},
    proto::generate_uuid,
};

#[derive(Clone)]
pub struct ArcChatApp {
    pub shared_app: Arc<Mutex<ChatApp>>,
}

fn main() -> Result<(), eframe::Error> {
    // Initialize ACK configuration at startup
    println!("🚀 Initialisation de DTChat");
    ack_config::initialize_ack_config();
    
    let config_path = match std::env::var("DTCHAT_CONFIG") {
        Ok(path) => path,
        Err(_) => {
            let default_path = "db/default.yaml".to_string();
            println!(
                "No DTCHAT_CONFIG environment variable found. Using default configuration: {default_path}"
            );
            default_path
        }
    };
    let config: AppConfigManager = AppConfigManager::load_yaml_from_file(&config_path);

    let shared_peers = config.peer_list;
    let shared_rooms = config.room_list;
    let local_peer = config.local_peer;
    let contact_plan = config.a_sabr;

    if !Path::new(&contact_plan).exists() {
        eprintln!("Contact plan missing !!!");
    }

    #[cfg(feature = "dev")]
    let mut now = Utc::now() - Duration::seconds(40);

    let prediction_config = match PredictionConfig::new(&contact_plan) {
        Ok(config) => Some(config),
        Err(e) => {
            eprintln!("Failed to create prediction_config: {e}");
            None
        }
    };

    #[cfg(feature = "dev")]
    let mut model = ChatModel::new(
        shared_peers.clone(),
        local_peer.clone(),
        shared_rooms.clone(),
        prediction_config,
    );

    #[cfg(not(feature = "dev"))]
    let model = ChatModel::new(
        shared_peers.clone(),
        local_peer.clone(),
        shared_rooms.clone(),
        prediction_config,
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

    match NetworkEngine::new(local_peer.clone(), shared_peers.clone()) {
        Ok(engine) => {
            let engine_arc = Arc::new(Mutex::new(engine));
            engine_arc.lock().unwrap().add_observer(model_arc.clone());
            
            // Store the engine for the app to use
            model_arc.lock().unwrap().set_network_engine(engine_arc.clone());
        }
        Err(e) => {
            eprintln!("Failed to initialize network engine: {e:?}");
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
