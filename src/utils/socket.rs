use crate::utils::config::Peer;
use crate::utils::message::ChatMessage;
use crate::utils::proto::{deserialize_message, serialize_message};
use libc::{self, c_int, sockaddr_storage, socklen_t};
use once_cell::sync::Lazy;
use serde::Deserialize;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::{self, Error, ErrorKind, Read, Write};
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
    eid_str: [u8; 126],           // 126 bytes - must match C struct
}

fn create_bp_sockaddr_with_string(eid_string: &str) -> io::Result<SockAddr> {
    let mut sockaddr_storage: libc::sockaddr_storage = unsafe { mem::zeroed() };

    unsafe {
        let sockaddr_ptr = &mut sockaddr_storage as *mut libc::sockaddr_storage as *mut libc::sockaddr;
        (*sockaddr_ptr).sa_family = AF_BP as u16;

        // Copy EID to sa_data (like your kernel module expects)
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
            _ => {
                return Err(Box::new(Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Unsupported socket type",
                )))
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
                                Ok((size)) => {
                                    println!(
                                        "UDP/BP received data on listening address {}",
                                        address
                                    );
                                    
                                    // ✅ FIX: Copy buffer data before async processing
                                    let data_copy = buffer[..size].to_vec();
                                    let new_controller_arc = Arc::clone(&controller_arc);
                                    
                                    TOKIO_RUNTIME.spawn(async move {
                                        let controller = new_controller_arc.lock().unwrap();
                                        let peers = controller.get_peers();

                                        if let Some(message) =
                                            deserialize_message(&data_copy, &peers)
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

    fn notify_observers(&self, message: ChatMessage) {
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
            match GenericSocket::new(endpoint) {
                Ok(mut sock) => {
                    if let Err(e) = sock.start_listener(controller_arc.clone()) {
                        eprintln!("Failed to start listener for {:?}: {}", endpoint, e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create socket for {:?}: {}", endpoint, e);
                    // Continue - don't crash the app
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
        self.send(&serialized)?;
        Ok(serialized.len())
    }
}
