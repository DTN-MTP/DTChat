pub mod endpoint;
pub mod socket;
pub mod engine;
pub mod encoding;

// Re-export commonly used types for convenience
pub use endpoint::{Endpoint, NetworkError, NetworkResult};
pub use socket::{GenericSocket, SocketController, SocketObserver, DefaultSocketController};
pub use engine::{NetworkEngine, NetworkStats, LoggingObserver};
pub use encoding::{MessageSerializer, MessageCodec, MessageFrame};
pub use crate::utils::proto::DeserializedMessage;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_module_exports() {
        // Simple test to ensure all exports are accessible
        let _endpoint = Endpoint::Tcp("127.0.0.1:8080".to_string());
        let _codec = MessageCodec::new();
    }
}
