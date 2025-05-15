use crate::utils::config::Peer;
use crate::utils::message::ChatMessage;
use crate::utils::proto::{deserialize_message, serialize_message};
use once_cell::sync::Lazy;
use serde::Deserialize;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::{self, Error, Read, Write};
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

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
}
pub struct GenericSocket {
    socket: Option<Socket>,
    eidpoint: Endpoint,
    sockaddr: Option<SocketAddr>,
    listening: bool,
    #[cfg(feature = "bp")]
    bp_socket: Option<crate::utils::bpsocket::BpSocket>,
}

impl GenericSocket {
    pub fn new(eid: &Endpoint) -> Result<Self, Box<dyn std::error::Error>> {
        match eid {
            Endpoint::Udp(addr) | Endpoint::Tcp(addr) => {
                let address: SocketAddr = addr.parse()?;
                let socket = match eid {
                    Endpoint::Udp(_) => Socket::new(
                        Domain::for_address(address),
                        Type::DGRAM,
                        Some(Protocol::UDP),
                    )?,
                    Endpoint::Tcp(_) => Socket::new(
                        Domain::for_address(address),
                        Type::STREAM,
                        Some(Protocol::TCP),
                    )?,
                    _ => unreachable!(),
                };

                Ok(Self {
                    socket: Some(socket),
                    eidpoint: eid.clone(),
                    sockaddr: Some(address),
                    listening: false,
                    #[cfg(feature = "bp")]
                    bp_socket: None,
                })
            },
            Endpoint::Bp(_) => {
                #[cfg(not(feature = "bp"))]
                {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "BP socket support requires 'bp' feature"
                    )));
                }
                
                #[cfg(feature = "bp")]
                {
                    Ok(Self {
                        socket: None,
                        eidpoint: eid.clone(),
                        sockaddr: None,
                        listening: false,
                        bp_socket: None,
                    })
                }
            }
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        match &self.eidpoint {
            Endpoint::Bp(addr) => {
                #[cfg(not(feature = "bp"))]
                {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "BP socket support requires 'bp' feature"
                    )));
                }
                
                #[cfg(feature = "bp")]
                {
                    // Lazily initialize BP socket if needed
                    if self.bp_socket.is_none() {
                        self.bp_socket = Some(crate::utils::bpsocket::BpSocket::new(&self.eidpoint)?);
                    }
                    
                    // Send using BP socket
                    if let Some(bp_socket) = &mut self.bp_socket {
                        bp_socket.send(data, addr)?;
                    }
                }
            }
            Endpoint::Udp(_) => {
                if let Some(socket) = &self.socket {
                    if let Some(sockaddr) = &self.sockaddr {
                        socket.send_to(data, &SockAddr::from(sockaddr.clone()))?;
                    }
                }
            }
            Endpoint::Tcp(_) => {
                if let Some(socket) = &self.socket {
                    if let Some(sockaddr) = &self.sockaddr {
                        let mut socket_clone = socket.try_clone()?;
                        socket_clone.connect(&SockAddr::from(sockaddr.clone()))?;
                        socket_clone.write_all(data)?;
                        socket_clone.flush()?;
                        socket_clone.shutdown(std::net::Shutdown::Both)?;
                    }
                }
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

        match &self.eidpoint {
            Endpoint::Bp(_) => {
                #[cfg(not(feature = "bp"))]
                {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "BP socket support requires 'bp' feature"
                    ));
                }
                
                #[cfg(feature = "bp")]
                {
                    // Lazily initialize BP socket if needed
                    if self.bp_socket.is_none() {
                        self.bp_socket = Some(crate::utils::bpsocket::BpSocket::new(&self.eidpoint)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?);
                    }
                    
                    // Start BP socket listener
                    if let Some(bp_socket) = &mut self.bp_socket {
                        return bp_socket.start_listener(controller_arc);
                    }
                    return Err(io::Error::new(io::ErrorKind::Other, "Failed to initialize BP socket"));
                }
            }
            Endpoint::Udp(_) | Endpoint::Tcp(_) => {
                if let Some(socket) = &self.socket {
                    if let Some(sockaddr) = &self.sockaddr {
                        let socket_clone = socket.try_clone()?;
                        socket_clone.set_nonblocking(true)?;
                        socket_clone.set_reuse_address(true)?;
                        socket_clone.bind(&SockAddr::from(sockaddr.clone()))?;
                        
                        match &self.eidpoint {
                            Endpoint::Udp(addr) => {
                                let address = addr.clone();
                                
                                TOKIO_RUNTIME.spawn_blocking({
                                    let mut socket = socket_clone.try_clone()?;
                                    move || {
                                        let mut buffer: [u8;1024] = [0; 1024];
                                        loop {
                                            match socket.read(&mut buffer) {
                                                Ok(size) => {
                                                    println!(
                                                        "UDP received data on listening address {}",
                                                        address
                                                    );
                                                    let new_controller_arc = Arc::clone(&controller_arc);
                                                    TOKIO_RUNTIME.spawn(async move {
                                                        let controller = new_controller_arc.lock().unwrap();
                                                        let peers = controller.get_peers();
                                                        
                                                        if let Some(message) =
                                                            deserialize_message(&buffer[..size], &peers)
                                                        {
                                                            controller.notify_observers(message);
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
                                socket_clone.listen(128)?;
                                TOKIO_RUNTIME.spawn_blocking({
                                    let socket = socket_clone.try_clone()?;
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
                            _ => unreachable!(),
                        }
                    }
                }
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

            if let Some(message) = deserialize_message(buffer_slice, &peers) {
                controller.notify_observers(message);
            }
        }
        Err(e) => {
            eprintln!("TCP Read Error: {}", e);
        }
    }
}

pub trait SocketObserver: Send + Sync {
    fn on_socket_event(&self, message: ChatMessage);
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

    pub fn notify_observers(&self, message: ChatMessage) {
        let observers_clone = self.observers.clone();
        let message_clone = message.clone();

        for observer in observers_clone {
            observer.on_socket_event(message_clone.clone());
        }
    }

    pub fn init_controller(
        local_peer: Peer,
        peers: Vec<Peer>,
    ) -> Result<Arc<Mutex<DefaultSocketController>>, Box<dyn std::error::Error>> {
        let mut controller = Self::new();
        controller.set_local_peer(local_peer.clone());
        controller.set_peers(peers);

        let controller_arc = Arc::new(Mutex::new(controller));

        for endpoint in &local_peer.endpoints {
            match endpoint {
                Endpoint::Bp(_) => {
                    #[cfg(not(feature = "bp"))]
                    {
                        println!("Warning: BP endpoint found but 'bp' feature is not enabled. Skipping endpoint.");
                        continue;
                    }
                    
                    #[cfg(feature = "bp")]
                    {
                        let mut sock = GenericSocket::new(endpoint)?;
                        sock.start_listener(controller_arc.clone())?;
                    }
                },
                _ => {
                    let mut sock = GenericSocket::new(endpoint)?;
                    sock.start_listener(controller_arc.clone())?;
                }
            }
        }

        Ok(controller_arc)
    }
}

pub trait SendingSocket: Send + Sync {
    fn send_message(&mut self, message: &ChatMessage) -> Result<usize, Box<dyn std::error::Error>>;
}

impl SendingSocket for GenericSocket {
    fn send_message(&mut self, message: &ChatMessage) -> Result<usize, Box<dyn std::error::Error>> {
        let serialized = serialize_message(message);
        
        // If the endpoint is BP, handle it separately
        match &self.eidpoint {
            Endpoint::Bp(_) => {
                #[cfg(not(feature = "bp"))]
                {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "BP socket support requires 'bp' feature"
                    )));
                }
                
                #[cfg(feature = "bp")]
                {
                    if let Some(bp_socket) = &mut self.bp_socket {
                        return bp_socket.send_message(message);
                    } else {
                        // Lazily initialize BP socket if needed
                        let mut bp_socket = crate::utils::bpsocket::BpSocket::new(&self.eidpoint)?;
                        let result = bp_socket.send_message(message);
                        self.bp_socket = Some(bp_socket);
                        return result;
                    }
                }
            }
            _ => {
                self.send(&serialized)?;
                Ok(serialized.len())
            }
        }
    }
}
