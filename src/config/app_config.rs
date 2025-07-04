use crate::domain::{Peer, Room};
use crate::utils::load_yaml_from_file;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub peer_list: Vec<Peer>,
    pub local_peer: Peer,
    pub room_list: Vec<Room>,
    pub a_sabr: String,
}

impl AppConfig {
    /// Crée une nouvelle instance d'AppConfig à partir d'un fichier YAML
    pub fn from_file(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        load_yaml_from_file(file_path)
    }

    /// Crée une nouvelle instance d'AppConfig à partir de la variable d'environnement ou du fichier par défaut
    pub fn from_env_or_default() -> Self {
        let config_path = match std::env::var("DTCHAT_CONFIG") {
            Ok(path) => {
                println!("📁 Configuration chargée depuis DTCHAT_CONFIG: {path}");
                path
            }
            Err(_) => {
                let default_path = "db/default.yaml".to_string();
                println!(
                    "📁 Variable DTCHAT_CONFIG non trouvée. Utilisation de la configuration par défaut: {default_path}"
                );
                default_path
            }
        };

        Self::from_file(&config_path).unwrap_or_else(|e| {
            panic!("❌ Échec du chargement de la configuration depuis '{config_path}': {e}");
        })
    }
}

/// Fonction d'initialisation pour la configuration de l'application
pub fn initialize_app_config() -> AppConfig {
    println!("🔧 Initialisation de la configuration de l'application");
    AppConfig::from_env_or_default()
}
