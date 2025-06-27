use crate::utils::ack::{self, AckConfig};
use crate::utils::config::Peer;
use crate::utils::message::ChatMessage;
use crate::utils::proto::{deserialize_message, serialize_message, DeserializedMessage};
use libc::{self, c_int};
use once_cell::sync::Lazy;
use serde::Deserialize;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::{self, Read, Write};
use std::mem;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use tokio::time::Duration;

const AF_BP: c_int = 28;

pub static TOKIO_RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "address")] // Internally tagged enum
pub enum Endpoint {
    Udp(String),
    Tcp(String),
    Bp(String),
}

impl Endpoint {
    pub fn to_string(&self) -> String {
        match self {
            Endpoint::Udp(s) => s.clone(),
            Endpoint::Tcp(s) => s.clone(),
            Endpoint::Bp(s) => s.clone(),
        }
    }

    /// Check if this endpoint is valid and can be used for socket operations
    pub fn is_valid(&self) -> bool {
        match self {
            Endpoint::Udp(addr) | Endpoint::Tcp(addr) => {
                // Try to parse the address to see if it's valid
                addr.parse::<std::net::SocketAddr>().is_ok()
            }
            Endpoint::Bp(addr) => {
                // Check if it's not a placeholder and follows basic BP EID format
                !addr.contains("PLACEHOLDER")
                    && !addr.is_empty()
                    && (addr.starts_with("ipn:") || addr.starts_with("dtn:"))
            }
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]

struct SockAddrBp {
    bp_family: libc::sa_family_t,
    eid_str: [u8; 126],
}

fn create_bp_sockaddr_with_string(eid_string: &str) -> io::Result<SockAddr> {
    // Validate the EID string format
    if eid_string.contains("PLACEHOLDER") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Cannot create socket with placeholder address: {}",
                eid_string
            ),
        ));
    }

    if eid_string.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "EID string cannot be empty",
        ));
    }

    if !eid_string.starts_with("ipn:") && !eid_string.starts_with("dtn:") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid BP EID format: {}", eid_string),
        ));
    }

    let mut sockaddr_bp = SockAddrBp {
        bp_family: AF_BP as libc::sa_family_t,
        eid_str: [0; 126],
    };

    // Copy EID string, ensuring null termination
    let bytes_to_copy = std::cmp::min(eid_string.len(), 125);
    sockaddr_bp.eid_str[..bytes_to_copy].copy_from_slice(&eid_string.as_bytes()[..bytes_to_copy]);

    // Convert to sockaddr_storage
    let mut sockaddr_storage: libc::sockaddr_storage = unsafe { mem::zeroed() };
    unsafe {
        std::ptr::copy_nonoverlapping(
            &sockaddr_bp as *const SockAddrBp as *const u8,
            &mut sockaddr_storage as *mut libc::sockaddr_storage as *mut u8,
            mem::size_of::<SockAddrBp>(),
        );
    }

    let addr_len = mem::size_of::<SockAddrBp>() as libc::socklen_t;
    let address = unsafe { SockAddr::new(sockaddr_storage, addr_len) };
    Ok(address)
}

pub struct GenericSocket {
    socket: Socket,
    eidpoint: Endpoint,
    sockaddr: SockAddr,
    listening: bool,
}

impl Clone for GenericSocket {
    fn clone(&self) -> Self {
        Self {
            socket: self.socket.try_clone().expect("Failed to clone socket"),
            eidpoint: self.eidpoint.clone(),
            sockaddr: self.sockaddr.clone(),
            listening: self.listening,
        }
    }
}
impl GenericSocket {
    pub fn new(eid: &Endpoint) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (domain, semtype, proto, address): (Domain, Type, Protocol, SockAddr) = match eid {
            Endpoint::Udp(addr) => {
                let std_sock = addr.parse()?;

                (
                    Domain::for_address(std_sock),
                    Type::DGRAM,
                    Protocol::UDP,
                    SockAddr::from(std_sock),
                )
            }
            Endpoint::Tcp(addr) => {
                let std_sock = addr.parse()?;
                (
                    Domain::for_address(std_sock),
                    Type::STREAM,
                    Protocol::TCP,
                    SockAddr::from(std_sock),
                )
            }
            Endpoint::Bp(addr) => (
                Domain::from(AF_BP),
                Type::DGRAM,
                Protocol::from(0),
                create_bp_sockaddr_with_string(addr)?,
            ),
        };

        let socket = Socket::new(domain, semtype, Some(proto))?;
        return Ok(Self {
            socket: socket,
            eidpoint: eid.clone(),
            sockaddr: address,
            listening: false,
        });
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self.eidpoint {
            Endpoint::Bp(_) | Endpoint::Udp(_) => {
                self.socket.send_to(data, &self.sockaddr.clone())?;
            }
            Endpoint::Tcp(_) => {
                self.socket.connect(&self.sockaddr.clone())?;
                self.socket.write_all(data)?;
                self.socket.flush()?;
                self.socket.shutdown(std::net::Shutdown::Both)?;
            }
        }

        Ok(())
    }

    pub fn start_listener(
        &mut self,
        controller_arc: Arc<Mutex<DefaultSocketController>>,
    ) -> io::Result<()> {
        if self.listening {
            return Ok(());
        }
        self.listening = true;

        self.socket.set_nonblocking(true)?;
        self.socket.set_reuse_address(true)?;
        self.socket.bind(&SockAddr::from(self.sockaddr.clone()))?;

        match &self.eidpoint {
            Endpoint::Udp(addr) | Endpoint::Bp(addr) => {
                let address = addr.clone();

                TOKIO_RUNTIME.spawn_blocking({
                    let mut socket = self.socket.try_clone()?; // Clone the socket for the async thread
                    move || {
                        let mut buffer: [u8; 1024] = [0; 1024];
                        loop {
                            match socket.read(&mut buffer) {
                                Ok(size) => {
                                    println!(
                                        "UDP/BP received data on listening address {}",
                                        address
                                    );
                                    let new_controller_arc = Arc::clone(&controller_arc);
                                    let address_clone = address.clone();
                                    TOKIO_RUNTIME.spawn(async move {
                                        let controller = new_controller_arc.lock().unwrap();
                                        let peers = controller.get_peers();
                                        let endpoint_type = if address_clone.starts_with("ipn:") || address_clone.starts_with("dtn:") {
                                            Endpoint::Bp(address_clone.clone())
                                        } else {
                                            Endpoint::Udp(address_clone.clone())
                                        };
                                        if let Some(deserialized) =
                                            deserialize_message(&buffer[..size], &peers)
                                        {
                                            match deserialized {
                                            DeserializedMessage::ChatMessage(message) => {
                                                println!("📨 Received message: '{}' from {}", message.text, message.sender.name);
                                                controller.send_ack_if_needed_with_endpoint_info(&message, Some(&endpoint_type));
                                                 #[cfg(feature = "delayed_ack")] {
                                                    let ctrl_clone = new_controller_arc.clone();
                                                    TOKIO_RUNTIME.spawn(async move {
                                                        sleep(Duration::from_millis(AckConfig::default().delay_duration_ms)).await;
                                                        ctrl_clone.lock().unwrap().notify_observers(message);
                                                    });
                                                }
                                                #[cfg(not(feature = "delayed_ack"))]
                                                controller.notify_observers(message);
                                            }
                                            DeserializedMessage::Ack { message_uuid, is_read, ack_time } => {
                                                println!("✅ Received ACK for message {} (read: {}) at {}",
                                                    message_uuid, is_read, ack_time.format("%H:%M:%S"));
                                                controller.handle_ack_received(&message_uuid, is_read, ack_time);
                                            }
                                        }
                                    }
                                });
                                }
                                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    thread::sleep(std::time::Duration::from_millis(10));
                                }
                                Err(e) => {
                                    eprintln!("UDP Error: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                });
            }
            Endpoint::Tcp(addr) => {
                let address = addr.clone();
                self.socket.listen(128)?;
                TOKIO_RUNTIME.spawn_blocking({
                    let socket = self.socket.try_clone()?; // Clone for async thread
                    move || loop {
                        match socket.accept() {
                            Ok((stream, _peer)) => {
                                println!("TCP received data on listening address {}", address);
                                let new_controller_arc = Arc::clone(&controller_arc);

                                TOKIO_RUNTIME.spawn(async move {
                                    handle_tcp_connection(stream.into(), new_controller_arc).await;
                                });
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                thread::sleep(std::time::Duration::from_millis(10));
                            }
                            Err(e) => {
                                eprintln!("TCP Error: {}", e);
                                break;
                            }
                        }
                    }
                });
            }
        }

        Ok(())
    }
}

async fn handle_tcp_connection(
    mut stream: std::net::TcpStream,
    controller_arc: Arc<Mutex<DefaultSocketController>>,
) {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(size) => {
            let buffer_slice = &buffer[..size];
            let controller = controller_arc.lock().unwrap();
            let peers = controller.get_peers();

            // Get the peer address to determine the endpoint
            let peer_addr = stream.peer_addr().ok();
            let tcp_endpoint = peer_addr.map(|addr| Endpoint::Tcp(addr.to_string()));

            if let Some(deserialized) = deserialize_message(buffer_slice, &peers) {
                match deserialized {
                    DeserializedMessage::ChatMessage(message) => {
                        println!(
                            "📨 TCP Received message: '{}' from {}",
                            message.text, message.sender.name
                        );
                        controller
                            .send_ack_if_needed_with_endpoint_info(&message, tcp_endpoint.as_ref());
                        #[cfg(feature = "delayed_ack")]
                        {
                            let ctrl_clone = controller_arc.clone();
                            TOKIO_RUNTIME.spawn(async move {
                                sleep(Duration::from_millis(
                                    AckConfig::default().delay_duration_ms,
                                ))
                                .await;
                                ctrl_clone.lock().unwrap().notify_observers(message);
                            });
                        }
                        #[cfg(not(feature = "delayed_ack"))]
                        controller.notify_observers(message);
                    }
                    DeserializedMessage::Ack {
                        message_uuid,
                        is_read,
                        ack_time,
                    } => {
                        println!(
                            "✅ TCP Received ACK for message {} (read: {}) at {}",
                            message_uuid,
                            is_read,
                            ack_time.format("%H:%M:%S")
                        );
                        controller.handle_ack_received(&message_uuid, is_read, ack_time);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("TCP Read Error: {}", e);
        }
    }
}

pub trait SocketObserver: Send + Sync {
    fn on_socket_event(&self, message: ChatMessage);
    fn on_ack_received(
        &self,
        message_uuid: &str,
        is_read: bool,
        ack_time: chrono::DateTime<chrono::Utc>,
    ) {
        // Default implementation does nothing
        let _ = (message_uuid, is_read, ack_time);
    }
}

pub trait SocketController: Send + Sync {
    fn add_observer(&mut self, observer: Arc<dyn SocketObserver + Send + Sync>);
}

impl SocketController for DefaultSocketController {
    fn add_observer(&mut self, observer: Arc<dyn SocketObserver + Send + Sync>) {
        self.observers.push(observer);
    }
}

pub struct DefaultSocketController {
    observers: Vec<Arc<dyn SocketObserver + Send + Sync>>,
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

    pub fn set_peers(&mut self, peers: Vec<Peer>) {
        self.peers = peers;
    }

    pub fn get_peers(&self) -> Vec<Peer> {
        self.peers.clone()
    }

    pub fn set_local_peer(&mut self, peer: Peer) {
        self.local_peer = Some(peer);
    }

    pub fn send_ack_if_needed_with_endpoint_info(
        &self,
        message: &ChatMessage,
        received_on_endpoint: Option<&Endpoint>,
    ) {
        if message.text.starts_with("[ACK]") {
            return;
        }

        println!("📤 Preparing to send ACK for message: '{}'", message.text);
        println!("🔍 Looking for sender with UUID: {}", message.sender.uuid);

        if let Some(local_peer) = &self.local_peer {
            // Afficher tous les peers disponibles pour déboguer
            println!("📋 Available peers:");
            for peer in &self.peers {
                println!("  - UUID: {}, Name: {}", peer.uuid, peer.name);
            }

            if let Some(sender_peer) = self.peers.iter().find(|p| p.uuid == message.sender.uuid) {
                println!(
                    "✅ Found sender peer: {} (UUID: {})",
                    sender_peer.name, sender_peer.uuid
                );

                let target_endpoint = self.choose_ack_endpoint(sender_peer, received_on_endpoint);
                println!(
                    "🎯 Sending ACK to {} via {}",
                    sender_peer.name,
                    target_endpoint.to_string()
                );

                let msg_clone = message.clone();
                let local_peer_uuid = local_peer.uuid.clone();

                // Send ACK to the chosen endpoint
                ack::send_ack_message_non_blocking(
                    &msg_clone,
                    &mut match GenericSocket::new(&target_endpoint) {
                        Ok(socket) => socket,
                        Err(e) => {
                            eprintln!("Failed to create socket for ACK: {}", e);
                            return;
                        }
                    },
                    &local_peer_uuid,
                    false, // Not read yet, just received
                    None,  // Use default config
                );
            } else {
                println!(
                    "❌ Sender peer {} not found in peer list",
                    message.sender.uuid
                );
                println!(
                    "📋 Available peer UUIDs: {:?}",
                    self.peers.iter().map(|p| &p.uuid).collect::<Vec<_>>()
                );
            }
        }
    }

    fn choose_ack_endpoint(
        &self,
        sender_peer: &Peer,
        received_on_endpoint: Option<&Endpoint>,
    ) -> Endpoint {
        // If we know which endpoint the message was received on, try to find a compatible one
        if let Some(received_endpoint) = received_on_endpoint {
            // For BP messages, prefer BP endpoints for ACK
            if matches!(received_endpoint, Endpoint::Bp(_)) {
                if let Some(bp_endpoint) = sender_peer
                    .endpoints
                    .iter()
                    .find(|ep| matches!(ep, Endpoint::Bp(_)))
                {
                    return bp_endpoint.clone();
                }
            }

            // For TCP/UDP, try to use the same protocol if available
            match received_endpoint {
                Endpoint::Tcp(_) => {
                    if let Some(tcp_endpoint) = sender_peer
                        .endpoints
                        .iter()
                        .find(|ep| matches!(ep, Endpoint::Tcp(_)))
                    {
                        return tcp_endpoint.clone();
                    }
                }
                Endpoint::Udp(_) => {
                    if let Some(udp_endpoint) = sender_peer
                        .endpoints
                        .iter()
                        .find(|ep| matches!(ep, Endpoint::Udp(_)))
                    {
                        return udp_endpoint.clone();
                    }
                }
                _ => {}
            }
        }

        // Fallback: prioritize BP > TCP > UDP for ACK reliability
        for endpoint in &sender_peer.endpoints {
            match endpoint {
                Endpoint::Bp(_) if endpoint.is_valid() => return endpoint.clone(),
                _ => {}
            }
        }
        for endpoint in &sender_peer.endpoints {
            match endpoint {
                Endpoint::Tcp(_) if endpoint.is_valid() => return endpoint.clone(),
                _ => {}
            }
        }
        for endpoint in &sender_peer.endpoints {
            match endpoint {
                Endpoint::Udp(_) if endpoint.is_valid() => return endpoint.clone(),
                _ => {}
            }
        }

        // Ultimate fallback: first valid endpoint
        sender_peer
            .endpoints
            .iter()
            .find(|ep| ep.is_valid())
            .unwrap_or(&sender_peer.endpoints[0])
            .clone()
    }

    fn notify_observers(&self, message: ChatMessage) {
        let observers_clone = self.observers.clone();
        let message_clone = message.clone();

        for observer in observers_clone {
            observer.on_socket_event(message_clone.clone());
        }
    }

    pub fn handle_ack_received(
        &self,
        message_uuid: &str,
        is_read: bool,
        ack_time: chrono::DateTime<chrono::Utc>,
    ) {
        println!("🔄 Processing ACK for message {}", message_uuid);
        // Notify observers about the ACK so they can update message status
        for observer in &self.observers {
            observer.on_ack_received(message_uuid, is_read, ack_time);
        }
    }

    pub fn init_controller(
        local_peer: Peer,
        peers: Vec<Peer>,
    ) -> Result<Arc<Mutex<DefaultSocketController>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut controller = Self::new();
        controller.set_local_peer(local_peer.clone());
        controller.set_peers(peers);

        let controller_arc = Arc::new(Mutex::new(controller));

        for endpoint in &local_peer.endpoints {
            // Skip invalid or placeholder endpoints
            if !endpoint.is_valid() {
                eprintln!("Skipping invalid or placeholder endpoint: {:?}", endpoint);
                continue;
            }

            match GenericSocket::new(endpoint) {
                Ok(mut sock) => {
                    if let Err(e) = sock.start_listener(controller_arc.clone()) {
                        eprintln!(
                            "Failed to start listener for endpoint {:?}: {}",
                            endpoint, e
                        );
                        // Continue with next endpoint instead of failing completely
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create socket for endpoint {:?}: {}", endpoint, e);
                    // Continue with next endpoint instead of failing
                }
            }
        }

        Ok(controller_arc)
    }
}

pub trait SendingSocket: Send + Sync {
    fn send_message(
        &mut self,
        message: &ChatMessage,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>>;
}

impl SendingSocket for GenericSocket {
    fn send_message(
        &mut self,
        message: &ChatMessage,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let serialized = serialize_message(message);
        self.send(&serialized)?;
        Ok(serialized.len())
    }
}
