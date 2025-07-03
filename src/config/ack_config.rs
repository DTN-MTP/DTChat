use once_cell::sync::Lazy;
use rand::Rng;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct AckConfig {
    pub delay_min_ms: u64,
    pub delay_max_ms: u64,
}

impl Default for AckConfig {
    fn default() -> Self {
        Self {
            delay_min_ms: 50,
            delay_max_ms: 200,
        }
    }
}

pub static ACK_CONFIG: Lazy<Arc<Mutex<AckConfig>>> =
    Lazy::new(|| Arc::new(Mutex::new(AckConfig::default())));

pub fn initialize_ack_config() {
    use std::io::{self, Write};

    println!("🔧 Configuration des délais d'ACK aléatoires");
    println!("Chaque ACK aura un délai différent dans la plage spécifiée.\n");

    // Demander la valeur minimale
    print!("Entrez le délai MINIMUM en millisecondes (défaut: 50ms): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    let min_delay = match io::stdin().read_line(&mut input) {
        Ok(_) => input.trim().parse::<u64>().unwrap_or(50),
        Err(_) => {
            println!("⚠️  Erreur de lecture, utilisation de la valeur par défaut");
            50
        }
    };

    print!("Entrez le délai MAXIMUM en millisecondes (défaut: 200ms): ");
    io::stdout().flush().unwrap();

    input.clear();
    let max_delay = match io::stdin().read_line(&mut input) {
        Ok(_) => {
            let parsed = input.trim().parse::<u64>().unwrap_or(200);
            if parsed < min_delay {
                println!(
                    "⚠️  Maximum ({}) < minimum ({}), ajustement automatique à {}ms",
                    parsed,
                    min_delay,
                    min_delay + 50
                );
                min_delay + 50
            } else {
                parsed
            }
        }
        Err(_) => {
            println!("⚠️  Erreur de lecture, utilisation de la valeur par défaut");
            200.max(min_delay + 50)
        }
    };

    {
        let mut config = ACK_CONFIG.lock().unwrap();
        config.delay_min_ms = min_delay;
        config.delay_max_ms = max_delay;
    }

    println!("✅ Délai ACK configuré: {min_delay}ms - {max_delay}ms (aléatoire)");
}

pub fn get_random_ack_delay_ms() -> u64 {
    let config = ACK_CONFIG.lock().unwrap();
    let mut rng = rand::rng();
    rng.random_range(config.delay_min_ms..=config.delay_max_ms)
}
