use once_cell::sync::Lazy;
use rand::Rng;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct AckConfig {
    pub delay_min_ms: u64,
    pub delay_max_ms: u64,
}

impl AckConfig {
    /// Crée une nouvelle configuration ACK avec des délais spécifiés
    pub fn new(delay_min_ms: u64, delay_max_ms: u64) -> Self {
        Self {
            delay_min_ms,
            delay_max_ms,
        }
    }

    /// Crée une configuration ACK basée sur un délai central avec une variance de ±25%
    #[cfg(feature = "ack-delay")]
    pub fn from_base_delay(base_delay_ms: u64) -> Self {
        let min_delay = (base_delay_ms as f64 * 0.75) as u64;
        let max_delay = (base_delay_ms as f64 * 1.25) as u64;
        Self::new(min_delay, max_delay)
    }

    /// Crée une configuration ACK à partir de la variable d'environnement ou de la valeur par défaut
    pub fn from_env_or_default() -> Self {
        #[cfg(feature = "ack-delay")]
        {
            let ack_delay = match std::env::var("DTCHAT_ACK_DELAY") {
                Ok(delay_str) => {
                    match delay_str.parse::<u64>() {
                        Ok(delay) => {
                            println!("✅ Délai ACK lu depuis DTCHAT_ACK_DELAY: {}ms", delay);
                            delay
                        }
                        Err(_) => {
                            println!("⚠️  Valeur DTCHAT_ACK_DELAY invalide '{}', utilisation de la valeur par défaut: 100ms", delay_str);
                            100
                        }
                    }
                }
                Err(_) => {
                    println!("⚠️  Variable DTCHAT_ACK_DELAY non trouvée, utilisation de la valeur par défaut: 100ms");
                    100
                }
            };
            Self::from_base_delay(ack_delay)
        }
        
        #[cfg(not(feature = "ack-delay"))]
        {
            Self::new(0, 0) // Délai de 0ms = envoi immédiat
        }
    }

    /// Génère un délai aléatoire dans la plage configurée
    pub fn get_random_delay_ms(&self) -> u64 {
        let mut rng = rand::rng();
        rng.random_range(self.delay_min_ms..=self.delay_max_ms)
    }
}

impl Default for AckConfig {
    fn default() -> Self {
        Self::new(50, 200)
    }
}

pub static ACK_CONFIG: Lazy<Arc<Mutex<AckConfig>>> =
    Lazy::new(|| Arc::new(Mutex::new(AckConfig::default())));

/// Fonction d'initialisation pour la configuration ACK
pub fn initialize_ack_config() {
    #[cfg(feature = "ack-delay")]
    println!("🔧 Configuration des délais d'ACK à partir des variables d'environnement");
    
    #[cfg(not(feature = "ack-delay"))]
    println!("🔧 Configuration ACK en mode immédiat (feature ack-delay désactivée)");
    
    let config = AckConfig::from_env_or_default();
    
    {
        let mut global_config = ACK_CONFIG.lock().unwrap();
        *global_config = config.clone();
    }

    #[cfg(feature = "ack-delay")]
    println!("✅ Délai ACK configuré: {}ms - {}ms (aléatoire)", 
             config.delay_min_ms, config.delay_max_ms);

}

/// Génère un délai ACK aléatoire basé sur la configuration globale
pub fn get_random_ack_delay_ms() -> u64 {
    let config = ACK_CONFIG.lock().unwrap();
    config.get_random_delay_ms()
}
