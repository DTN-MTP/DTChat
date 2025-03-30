use once_cell::sync::Lazy;
use serde::Deserialize;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::fmt::Debug;
use std::io::{self, Error, Read, Write};
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;
use tokio::task;

use super::config::Peer;

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
    observers: Vec<Arc<dyn SocketObserver + Send + Sync>>,
    socket: Socket,
    eidpoint: Endpoint,
    sockaddr: SocketAddr,
    listening: bool,
}
impl GenericSocket {
    pub fn new(eid: &Endpoint) -> Result<Self, Box<dyn std::error::Error>> {
        let address: SocketAddr = match eid {
            Endpoint::Udp(addr) | Endpoint::Tcp(addr) => addr.parse()?,
            Endpoint::Bp(_addr) => todo!(), // I don't expect parse()? to work, sockaddr must have an addr familly,
        };

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
            Endpoint::Bp(_) => Socket::new(Domain::from(28), Type::DGRAM, Some(Protocol::from(0)))?,
        };

        return Ok(Self {
            observers: Vec::new(),
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
                    .send_to(data, &SockAddr::from(self.sockaddr.clone()))?;
            }
            Endpoint::Tcp(_) => {
                self.socket
                    .connect(&SockAddr::from(self.sockaddr.clone()))?;
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
                    let socket = self.socket.try_clone()?; // Clone the socket for the async thread
                    move || {
                        let mut buffer = [MaybeUninit::uninit(); 1024];
                        loop {
                            match socket.recv_from(&mut buffer) {
                                Ok((_size, _senderr)) => {
                                    println!(
                                        "UDP/BP received data on listening address {}",
                                        address
                                    );
                                    let new_controller_arc = Arc::clone(&controller_arc);
                                    let buffer_initialized =
                                        unsafe { std::mem::transmute::<_, [u8; 1024]>(buffer) };
                                    TOKIO_RUNTIME.spawn(async move {
                                        let guard = new_controller_arc.lock().unwrap();
                                        guard.notify_observers(
                                            &String::from_utf8_lossy(&buffer_initialized),
                                            "0".to_string(),
                                        ); // TODO CHANGE THAT
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
                                println!(
                                    "TCP received data on listening address {}",
                                    address
                                );
                                let new_controller_arc = Arc::clone(&controller_arc);
                                thread::spawn(move || {
                                    handle_tcp_connection(stream.into(), new_controller_arc)
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
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Unsupported socket type",
                ))
            }
        }

        Ok(())
    }
}

fn handle_tcp_connection(
    mut stream: std::net::TcpStream,
    controller_arc: Arc<Mutex<DefaultSocketController>>,
) {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(size) => {
            let text = String::from_utf8_lossy(&buffer[..size]).to_string();
            TOKIO_RUNTIME.spawn(async move {
                let guard = controller_arc.lock().unwrap();
                guard.notify_observers(&text, "0".to_string()); // TODO CHANGE THAT
            });
        }
        Err(e) => {
            eprintln!("TCP Read Error: {}", e);
        }
    }
}

pub trait SocketObserver: Send + Sync {
    fn on_socket_event(&self, text: &str, sender: String);
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
}

impl DefaultSocketController {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
        }
    }

    fn notify_observers(&self, text: &str, sender_id: String) {
        let observers_clone = self.observers.clone();
        let text_owned = text.trim().to_string();

        TOKIO_RUNTIME.spawn(async move {
            for observer in observers_clone {
                observer.on_socket_event(&text_owned, sender_id.clone());
            }
        });
    }

    pub fn init_controller(
        local_peer: Peer,
    ) -> Result<Arc<Mutex<DefaultSocketController>>, Box<dyn std::error::Error>> {
        let mut controller = Self::new();
        let controller_arc = Arc::new(Mutex::new(controller));

        for endpoint in &local_peer.endpoints {
            let mut sock = GenericSocket::new(endpoint).unwrap();
            sock.start_listener(controller_arc.clone());
        }

        Ok(controller_arc)
    }
}
