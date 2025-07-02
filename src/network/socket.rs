use crate::network::endpoint::{create_bp_sockaddr, Endpoint, NetworkError, NetworkResult};
use crate::network::encoding::MessageCodec;
use crate::utils::{config::Peer, message::ChatMessage, proto::DeserializedMessage};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::{
    io::{self, Read, Write},
    net::SocketAddr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::runtime::Runtime;
use once_cell::sync::Lazy;

const AF_BP: libc::c_int = 28;

pub static TOKIO_RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

/// Trait for observing socket events
pub trait SocketObserver: Send + Sync {
    fn on_message_received(&self, message: ChatMessage);
    fn on_ack_received(
        &self,
        message_uuid: &str,
        is_read: bool,
        ack_time: chrono::DateTime<chrono::Utc>,
    );
}

/// Trait for controlling socket operations
pub trait SocketController: Send + Sync {
    fn add_observer(&mut self, observer: Arc<dyn SocketObserver>);
    fn get_peers(&self) -> Vec<Peer>;
    fn get_local_peer(&self) -> Option<&Peer>;
    fn get_observers(&self) -> Vec<Arc<dyn SocketObserver>>;
    fn notify_observers(&self, message: ChatMessage);
    fn handle_ack_received(&self, message_uuid: &str, is_read: bool, ack_time: chrono::DateTime<chrono::Utc>);
}

/// Configuration for network socket operations
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SocketConfig {
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub buffer_size: usize,
    pub max_connections: usize,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(10),
            buffer_size: 8192,
            max_connections: 100,
        }
    }
}

/// Generic network socket implementation
pub struct GenericSocket {
    socket: Socket,
    endpoint: Endpoint,
    sockaddr: SockAddr,
    listening: bool,
    config: SocketConfig,
}

impl GenericSocket {
    pub fn new(endpoint: Endpoint) -> NetworkResult<Self> {
        Self::with_config(endpoint, SocketConfig::default())
    }

    pub fn with_config(endpoint: Endpoint, config: SocketConfig) -> NetworkResult<Self> {
        let (domain, socket_type, protocol, address) = match &endpoint {
            Endpoint::Udp(addr) => {
                let socket_addr: SocketAddr = addr.parse()
                    .map_err(|e: std::net::AddrParseError| NetworkError::AddressParseError(e.to_string()))?;
                (
                    Domain::for_address(socket_addr),
                    Type::DGRAM,
                    Protocol::UDP,
                    SockAddr::from(socket_addr),
                )
            }
            Endpoint::Tcp(addr) => {
                let socket_addr: SocketAddr = addr.parse()
                    .map_err(|e: std::net::AddrParseError| NetworkError::AddressParseError(e.to_string()))?;
                (
                    Domain::for_address(socket_addr),
                    Type::STREAM,
                    Protocol::TCP,
                    SockAddr::from(socket_addr),
                )
            }
            Endpoint::Bp(addr) => (
                Domain::from(AF_BP),
                Type::DGRAM,
                Protocol::from(0),
                create_bp_sockaddr(addr)?,
            ),
        };

        let socket = Socket::new(domain, socket_type, Some(protocol))?;
        
        Ok(Self {
            socket,
            endpoint,
            sockaddr: address,
            listening: false,
            config,
        })
    }

    pub fn send(&mut self, data: &[u8]) -> NetworkResult<usize> {
        match &self.endpoint {
            Endpoint::Udp(_) | Endpoint::Bp(_) => {
                self.socket.send_to(data, &self.sockaddr)?;
                Ok(data.len())
            }
            Endpoint::Tcp(_) => {
                self.socket.connect(&self.sockaddr)?;
                self.socket.write_all(data)?;
                self.socket.flush()?;
                self.socket.shutdown(std::net::Shutdown::Both)?;
                Ok(data.len())
            }
        }
    }

    pub fn start_listener<C>(&mut self, controller: Arc<Mutex<C>>) -> NetworkResult<()> 
    where
        C: SocketController + 'static,
    {
        if self.listening {
            return Ok(());
        }
        
        self.listening = true;
        self.socket.set_nonblocking(true)?;
        self.socket.set_reuse_address(true)?;
        self.socket.bind(&self.sockaddr)?;

        match &self.endpoint {
            Endpoint::Udp(addr) | Endpoint::Bp(addr) => {
                self.start_datagram_listener(addr.clone(), controller)
            }
            Endpoint::Tcp(addr) => {
                self.start_stream_listener(addr.clone(), controller)
            }
        }
    }

    fn start_datagram_listener<C>(&mut self, address: String, controller: Arc<Mutex<C>>) -> NetworkResult<()>
    where
        C: SocketController + 'static,
    {
        let mut socket = self.socket.try_clone()?;
        let endpoint = self.endpoint.clone();
        
        TOKIO_RUNTIME.spawn_blocking(move || {
            let mut buffer = vec![0u8; 8192];
            loop {
                match socket.read(&mut buffer) {
                    Ok(size) => {
                        println!("UDP/BP received {} bytes on {}", size, address);
                        
                        let controller_clone = Arc::clone(&controller);
                        let endpoint_clone = endpoint.clone();
                        let data = buffer[0..size].to_vec();
                        
                        TOKIO_RUNTIME.spawn(async move {
                            Self::handle_received_data(data, controller_clone, endpoint_clone).await;
                        });
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => {
                        eprintln!("UDP/BP Error: {}", e);
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }

    fn start_stream_listener<C>(&mut self, address: String, controller: Arc<Mutex<C>>) -> NetworkResult<()>
    where
        C: SocketController + 'static,
    {
        self.socket.listen(128)?;
        let socket = self.socket.try_clone()?;
        
        TOKIO_RUNTIME.spawn_blocking(move || {
            loop {
                match socket.accept() {
                    Ok((stream, peer_addr)) => {
                        println!("TCP connection from {:?} on {}", peer_addr, address);
                        
                        let controller_clone = Arc::clone(&controller);
                        TOKIO_RUNTIME.spawn(async move {
                            Self::handle_tcp_connection(stream.into(), controller_clone).await;
                        });
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => {
                        eprintln!("TCP Error: {}", e);
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }

    async fn handle_tcp_connection<C>(mut stream: std::net::TcpStream, controller: Arc<Mutex<C>>)
    where
        C: SocketController + 'static,
    {
        let mut buffer = vec![0u8; 8192];
        match stream.read(&mut buffer) {
            Ok(size) => {
                println!("TCP received {} bytes", size);
                Self::handle_received_data(buffer[0..size].to_vec(), controller, 
                    Endpoint::Tcp("unknown".to_string())).await;
            }
            Err(e) => {
                eprintln!("TCP Read Error: {}", e);
            }
        }
    }

    async fn handle_received_data<C>(data: Vec<u8>, controller: Arc<Mutex<C>>, _endpoint: Endpoint)
    where
        C: SocketController + 'static,
    {
        let peers = {
            let ctrl = controller.lock().unwrap();
            ctrl.get_peers()
        };

        let codec = MessageCodec::new();
        if let Ok(Some(deserialized)) = codec.decode(&data, &peers) {
            let ctrl = controller.lock().unwrap();
            match deserialized {
                DeserializedMessage::ChatMessage(message) => {
                    println!("📨 Received message: '{}' from {}", message.text, message.sender.name);
                    ctrl.notify_observers(message);
                }
                DeserializedMessage::Ack { message_uuid, is_read, ack_time } => {
                    println!("✅ Received ACK for message {} (read: {}) at {}",
                        message_uuid, is_read, ack_time.format("%H:%M:%S"));
                    ctrl.handle_ack_received(&message_uuid, is_read, ack_time);
                }
            }
        } else {
            eprintln!("Failed to deserialize received data");
        }
    }

    /// Send a chat message
    pub fn send_message(&mut self, message: &ChatMessage) -> NetworkResult<usize> {
        let codec = MessageCodec::new();
        let serialized = codec.encode(message)?;
        self.send(&serialized)
    }

}

impl Clone for GenericSocket {
    fn clone(&self) -> Self {
        Self {
            socket: self.socket.try_clone().expect("Failed to clone socket"),
            endpoint: self.endpoint.clone(),
            sockaddr: self.sockaddr.clone(),
            listening: self.listening,
            config: self.config.clone(),
        }
    }
}

/// Default implementation of SocketController
pub struct DefaultSocketController {
    observers: Vec<Arc<dyn SocketObserver>>,
    local_peer: Option<Peer>,
    peers: Vec<Peer>,
}

impl DefaultSocketController {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
            local_peer: None,
            peers: Vec::new(),
        }
    }

    pub fn set_local_peer(&mut self, peer: Peer) {
        self.local_peer = Some(peer);
    }

    pub fn set_peers(&mut self, peers: Vec<Peer>) {
        self.peers = peers;
    }

    /// Initialize and start listeners for all endpoints
    pub fn init_controller(
        local_peer: Peer,
        peers: Vec<Peer>,
    ) -> NetworkResult<Arc<Mutex<Self>>> {
        let mut controller = Self::new();
        controller.set_local_peer(local_peer.clone());
        controller.set_peers(peers);

        let controller_arc = Arc::new(Mutex::new(controller));

        for endpoint in &local_peer.endpoints {
            if !endpoint.is_valid() {
                eprintln!("Skipping invalid endpoint: {:?}", endpoint);
                continue;
            }

            match GenericSocket::new(endpoint.clone()) {
                Ok(mut socket) => {
                    if let Err(e) = socket.start_listener(Arc::clone(&controller_arc)) {
                        eprintln!("Failed to start listener for {:?}: {}", endpoint, e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create socket for {:?}: {}", endpoint, e);
                }
            }
        }

        Ok(controller_arc)
    }
}

impl SocketController for DefaultSocketController {
    fn add_observer(&mut self, observer: Arc<dyn SocketObserver>) {
        self.observers.push(observer);
    }

    fn get_peers(&self) -> Vec<Peer> {
        self.peers.clone()
    }

    fn get_local_peer(&self) -> Option<&Peer> {
        self.local_peer.as_ref()
    }

    fn get_observers(&self) -> Vec<Arc<dyn SocketObserver>> {
        self.observers.clone()
    }

    fn notify_observers(&self, message: ChatMessage) {
        for observer in &self.observers {
            observer.on_message_received(message.clone());
        }
    }

    fn handle_ack_received(&self, message_uuid: &str, is_read: bool, ack_time: chrono::DateTime<chrono::Utc>) {
        for observer in &self.observers {
            observer.on_ack_received(message_uuid, is_read, ack_time);
        }
    }
}


