use crate::utils::config::Peer;
use crate::utils::message::ChatMessage;
use crate::utils::proto::{deserialize_message, serialize_message};
use libc::{self, c_int};
use once_cell::sync::Lazy;
use serde::Deserialize;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::{self, Error, Read, Write};
use std::{mem, ptr};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

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
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct SockAddrBp {
    bp_family: libc::sa_family_t, // 2 bytes
    bp_agent_id: u8,              // 1 byte
    _pad: [u8; 13],               // Padding to reach 16 bytes total
}

fn create_bp_sockaddr_with_string(eid_string: &str) -> io::Result<SockAddr> {
    let mut sockaddr_storage: libc::sockaddr_storage = unsafe { mem::zeroed() };

    // Set the family
    unsafe {
        let sockaddr_ptr = &mut sockaddr_storage as *mut libc::sockaddr_storage as *mut libc::sockaddr;
        (*sockaddr_ptr).sa_family = AF_BP as u16;

        // Copy the string to sa_data (similar to your C strncpy)
        let sa_data_ptr = (*sockaddr_ptr).sa_data.as_mut_ptr();
        let bytes_to_copy = std::cmp::min(eid_string.len(), (*sockaddr_ptr).sa_data.len() - 1);

        std::ptr::copy_nonoverlapping(
            eid_string.as_ptr(),
            sa_data_ptr as *mut u8,
            bytes_to_copy,
        );

        // Null terminate
        *((sa_data_ptr as *mut u8).add(bytes_to_copy)) = 0;
    }

    let addr_len = mem::size_of::<libc::sockaddr>() as libc::socklen_t;
    let address = unsafe { SockAddr::new(sockaddr_storage, addr_len) };
    Ok(address)
}

pub struct GenericSocket {
    socket: Socket,
    eidpoint: Endpoint,
    sockaddr: SockAddr,
    listening: bool,
}
impl GenericSocket {
    pub fn new(eid: &Endpoint) -> Result<Self, Box<dyn std::error::Error>> {
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

    pub fn send(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        match self.eidpoint {
            Endpoint::Bp(_) | Endpoint::Udp(_) => {
                self.socket
                    .send_to(data, &self.sockaddr.clone())?;
            }
            Endpoint::Tcp(_) => {
                self.socket
                    .connect(&self.sockaddr.clone())?;
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

                // Check for pending messages on startup using the controller method
                {
                    let controller = controller_arc.lock().unwrap();
                    let mut socket_clone = self.socket.try_clone()?;
                    controller.check_pending_messages(&mut socket_clone, &address);
                }

                TOKIO_RUNTIME.spawn_blocking({
                    let mut socket = self.socket.try_clone()?;
                    let controller_clone = Arc::clone(&controller_arc);
                    move || {
                        let mut buffer: [u8; 8192] = [0; 8192];
                        loop {
                            match socket.read(&mut buffer) {
                                Ok(size) => {
                                    let controller = controller_clone.lock().unwrap();
                                    controller.handle_incoming_data(&buffer, size, &address);
                                }
                                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    thread::sleep(std::time::Duration::from_millis(100));
                                }
                                Err(e) => {
                                    eprintln!("Socket Error on {}: {}", address, e);
                                    thread::sleep(std::time::Duration::from_millis(1000));
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
                    let socket = self.socket.try_clone()?;
                    let controller_clone = Arc::clone(&controller_arc);
                    move || loop {
                        match socket.accept() {
                            Ok((stream, _peer)) => {
                                let controller_for_connection = Arc::clone(&controller_clone);
                                TOKIO_RUNTIME.spawn(async move {
                                    handle_tcp_connection(stream.into(), controller_for_connection).await;
                                });
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                thread::sleep(std::time::Duration::from_millis(100));
                            }
                            Err(e) => {
                                eprintln!("TCP Socket Error on {}: {}", address, e);
                                thread::sleep(std::time::Duration::from_millis(1000));
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
    let mut buffer = [0; 8192];
    match stream.read(&mut buffer) {
        Ok(size) => {
            let controller = controller_arc.lock().unwrap();
            controller.handle_incoming_data(&buffer, size, "tcp-connection");
        }
        Err(e) => {
            eprintln!("TCP Connection Read Error: {}", e);
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

    fn notify_observers(&self, message: ChatMessage) {
        let observers_clone = self.observers.clone();
        let message_clone = message.clone();

        for observer in observers_clone {
            observer.on_socket_event(message_clone.clone());
        }
    }

    // New method: Handle incoming message data consistently
    pub fn handle_incoming_data(&self, buffer: &[u8], size: usize, source: &str) {
        if size > 0 {
            println!("Received data from {}: {} bytes", source, size);
            let peers = self.get_peers();
            if let Some(message) = deserialize_message(&buffer[..size], &peers) {
                self.notify_observers(message);
            }
        }
    }

    // New method: Check for pending messages on startup
    pub fn check_pending_messages(&self, socket: &mut Socket, endpoint_addr: &str) {
        let mut buffer: [u8; 8192] = [0; 8192];
        match socket.read(&mut buffer) {
            Ok(size) => {
                self.handle_incoming_data(&buffer, size, &format!("startup-{}", endpoint_addr));
            }
            Err(_) => {
            }
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
            let mut sock = GenericSocket::new(endpoint).unwrap();
            sock.start_listener(controller_arc.clone())?;
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
        self.send(&serialized)?;
        Ok(serialized.len())
    }
}
