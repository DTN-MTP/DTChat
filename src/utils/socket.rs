use crate::utils::config::Peer;
use crate::utils::message::Message;
use crate::utils::proto::{deserialize_message, serialize_message};
use once_cell::sync::Lazy;
use serde::Deserialize;
use socket2::{Domain, SockAddr, Socket, Type};
use std::io::Error as IoError;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::runtime::Runtime;
use tokio::time::sleep;

pub static TOKIO_RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

#[cfg(feature = "add_delay")]
async fn maybe_delay() {
    sleep(Duration::from_secs(1)).await;
}

#[cfg(not(feature = "add_delay"))]
async fn maybe_delay() {}

fn ephemeral_local_addr_for(addr: &SocketAddr) -> SocketAddr {
    if addr.is_ipv4() {
        "0.0.0.0:0".parse().unwrap()
    } else {
        "[::]:0".parse().unwrap()
    }
}

#[derive(Debug)]
pub enum SocketError {
    Io(IoError),
    Custom(String),
    Serialization(String),
}

impl From<IoError> for SocketError {
    fn from(err: IoError) -> Self {
        SocketError::Io(err)
    }
}

impl From<std::net::AddrParseError> for SocketError {
    fn from(err: std::net::AddrParseError) -> Self {
        SocketError::Custom(err.to_string())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "address")] // Internally tagged enum
pub enum Endpoint {
    Udp(String),
    Tcp(String),
    #[cfg(feature = "bp")]
    Bp(String),
}

impl Endpoint {
    pub fn to_string(&self) -> String {
        match self {
            Endpoint::Udp(s) => s.clone(),
            Endpoint::Tcp(s) => s.clone(),
            #[cfg(feature = "bp")]
            Endpoint::Bp(s) => s.clone(),
        }
    }
}
pub trait SendingSocket: Send + Sync {
    fn send(&mut self, message: &Message) -> Result<usize, SocketError>;
}

pub struct UdpSendingSocket {
    socket: UdpSocket,
    addr: SocketAddr,
    peers: Vec<Peer>,
}

impl UdpSendingSocket {
    pub async fn new(address: &str, peers: Vec<Peer>) -> Result<Self, SocketError> {
        let addr: SocketAddr = address.parse()?;
        let local = ephemeral_local_addr_for(&addr);

        let std_socket = Socket::new(Domain::for_address(addr), Type::DGRAM, None)?;
        std_socket.bind(&SockAddr::from(local))?;
        std_socket.set_nonblocking(true)?;
        let udp_socket = UdpSocket::from_std(std_socket.into())?;

        Ok(Self {
            socket: udp_socket,
            addr,
            peers,
        })
    }
}

impl SendingSocket for UdpSendingSocket {
    fn send(&mut self, message: &Message) -> Result<usize, SocketError> {
        TOKIO_RUNTIME.block_on(async {
            let serialized = serialize_message(message);
            let bytes_sent = self.socket.send_to(&serialized, &self.addr).await?;
            println!(
                "UDP: successfully sent {} bytes to {}",
                bytes_sent, self.addr
            );

            Ok(bytes_sent)
        })
    }
}

pub struct TcpSendingSocket {
    addr: SocketAddr,
    peers: Vec<Peer>,
}

impl TcpSendingSocket {
    pub async fn new(address: &str, peers: Vec<Peer>) -> Result<Self, SocketError> {
        let addr: SocketAddr = address.parse()?;
        Ok(Self { addr, peers })
    }
}

impl SendingSocket for TcpSendingSocket {
    fn send(&mut self, message: &Message) -> Result<usize, SocketError> {
        TOKIO_RUNTIME.block_on(async {
            let mut stream = TcpStream::connect(self.addr).await.map_err(|e| {
                println!("TCP: failed to connect to {}: {:?}", self.addr, e);
                SocketError::Io(e)
            })?;

            let serialized = serialize_message(message);
            stream.write_all(&serialized).await?;
            stream.shutdown().await?;

            println!("TCP: successfully sent {} bytes to remote", serialized.len());
            Ok(serialized.len())
        })
    }
}

#[cfg(feature = "bp")]
mod bp_socket {
    use super::*;
    const AF_BP: i32 = 28;

    pub struct BpSendingSocket {
        socket: UdpSocket,
        addr: SocketAddr,
        peers: Vec<Peer>,
    }

    impl BpSendingSocket {
        pub async fn new(address: &str, peers: Vec<Peer>) -> Result<Self, SocketError> {
            let addr: SocketAddr = address.parse()?;
            let local = ephemeral_local_addr_for(&addr);

            let std_socket = Socket::new_raw(Domain::from_raw(AF_BP), Type::DGRAM, None)?;
            std_socket.bind(&SockAddr::from(local))?;
            std_socket.set_nonblocking(true)?;
            let socket = UdpSocket::from_std(std_socket.into())?;

            Ok(Self { socket, addr, peers })
        }
    }

    impl SendingSocket for BpSendingSocket {
        fn send(&mut self, message: &Message) -> Result<usize, SocketError> {
            TOKIO_RUNTIME.block_on(async {
                let serialized = serialize_message(message);
                Ok(serialized.len())
            })
        }
    }
}

pub fn create_sending_socket(
    protocol: Endpoint,
    peers: Vec<Peer>,
) -> Result<Box<dyn SendingSocket>, SocketError> {
    match protocol {
        Endpoint::Udp(addr) => {
            let socket = TOKIO_RUNTIME.block_on(async { 
                UdpSendingSocket::new(&addr, peers).await 
            })?;
            Ok(Box::new(socket))
        }
        Endpoint::Tcp(addr) => {
            let socket = TOKIO_RUNTIME.block_on(async {
                TcpSendingSocket::new(&addr, peers).await
            })?;
            Ok(Box::new(socket))
        }
        #[cfg(feature = "bp")]
        Endpoint::Bp(addr) => {
            let socket = TOKIO_RUNTIME.block_on(async {
                BpSendingSocket::new(&addr, peers).await
            })?;
            Ok(Box::new(socket))
        }
    }
}

pub async fn start_udp_listener(address: &str) -> Result<UdpSocket, SocketError> {
    let addr: SocketAddr = address.parse()?;
    let socket = UdpSocket::bind(addr).await?;
    Ok(socket)
}

pub async fn start_tcp_listener(address: &str) -> Result<TcpListener, SocketError> {
    let addr: SocketAddr = address.parse()?;
    let listener = TcpListener::bind(addr).await?;
    Ok(listener)
}

pub trait SocketObserver: Send + Sync {
    fn on_socket_event(&self, text: &str, sender: Peer);
}

pub trait SocketController: Send + Sync {
    fn add_observer(&mut self, observer: Arc<dyn SocketObserver + Send + Sync>);
}

pub struct DefaultSocketController {
    observers: Vec<Arc<dyn SocketObserver + Send + Sync>>,
    local_peer: Option<Peer>,
    peers: Vec<Peer>,
    udp_socket: Option<UdpSocket>,
    tcp_listener: Option<TcpListener>,
}

impl DefaultSocketController {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
            local_peer: None,
            peers: Vec::new(),
            udp_socket: None,
            tcp_listener: None,
        }
    }

    fn notify_observers(&self, text: &str, sender: Peer) {
        let observers_clone = self.observers.clone();
        let text_owned = text.trim().to_string();

        TOKIO_RUNTIME.spawn(async move {
            for observer in observers_clone {
                observer.on_socket_event(&text_owned, sender.clone());
            }
        });
    }

    async fn run_udp_listener(
        controller_arc: Arc<Mutex<DefaultSocketController>>,
        socket: UdpSocket,
    ) {
        let mut buf = [0u8; 4096];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((n, addr)) => {
                    let controller = Arc::clone(&controller_arc);
                    let buf_data = buf[..n].to_vec();
                    
                    TOKIO_RUNTIME.spawn(async move {
                        let guard = controller.lock().unwrap();
                        let peers = guard.peers.clone();
                        
                        if let Some(message) = deserialize_message(&buf_data, &peers) {
                            guard.notify_observers(&message.text, message.sender);
                        } else {
                            println!("UDP: failed to deserialize message from {}", addr);
                        }
                    });
                }
                Err(e) => {
                    println!("UDP: receiving error {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn run_tcp_listener(
        controller_arc: Arc<Mutex<DefaultSocketController>>,
        listener: TcpListener,
    ) {
        loop {
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    println!("TCP: accepted connection from {}", addr);
                    let mut buf = Vec::new();
                    
                    match stream.read_to_end(&mut buf).await {
                        Ok(n) if n > 0 => {
                            let controller = Arc::clone(&controller_arc);
                            
                            TOKIO_RUNTIME.spawn(async move {
                                let guard = controller.lock().unwrap();
                                let peers = guard.peers.clone();
                                
                                if let Some(message) = deserialize_message(&buf, &peers) {
                                    guard.notify_observers(&message.text, message.sender);
                                } else {
                                    println!("TCP: failed to deserialize message from {}", addr);
                                }
                            });
                        }
                        Ok(_) => {
                            println!("TCP: zero bytes read from {}, closing", addr);
                        }
                        Err(e) => {
                            println!("TCP: read error {:?} from {}", e, addr);
                        }
                    }
                }
                Err(e) => {
                    println!("TCP: accept error {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    pub fn init_controller(
        local_peer: Peer,
        peers: Vec<Peer>,
    ) -> Result<Arc<Mutex<DefaultSocketController>>, SocketError> {
        let mut udp_socket = None;
        let mut tcp_listener = None;

        for endpoint in &local_peer.endpoints {
            match endpoint {
                Endpoint::Udp(addr) => {
                    udp_socket =
                        Some(TOKIO_RUNTIME.block_on(async { start_udp_listener(&addr).await })?)
                }
                Endpoint::Tcp(addr) => {
                    tcp_listener =
                        Some(TOKIO_RUNTIME.block_on(async { start_tcp_listener(&addr).await })?)
                }
                #[cfg(feature = "bp")]
                Endpoint::Bp(_) => {}
            }
        }

        let mut controller = Self::new();
        controller.local_peer = Some(local_peer);
        controller.peers = peers;
        controller.udp_socket = udp_socket;
        controller.tcp_listener = tcp_listener;

        let controller_arc = Arc::new(Mutex::new(controller));

        if let Some(_) = &controller_arc.lock().unwrap().udp_socket {
            let arc_clone = Arc::clone(&controller_arc);
            TOKIO_RUNTIME.spawn(async move {
                let socket = arc_clone.lock().unwrap().udp_socket.take().unwrap();
                DefaultSocketController::run_udp_listener(arc_clone, socket).await;
            });
        }

        if let Some(_) = &controller_arc.lock().unwrap().tcp_listener {
            let arc_clone = Arc::clone(&controller_arc);
            TOKIO_RUNTIME.spawn(async move {
                let listener = arc_clone.lock().unwrap().tcp_listener.take().unwrap();
                DefaultSocketController::run_tcp_listener(arc_clone, listener).await;
            });
        }

        Ok(controller_arc)
    }
}

impl SocketController for DefaultSocketController {
    fn add_observer(&mut self, observer: Arc<dyn SocketObserver + Send + Sync>) {
        self.observers.push(observer);
    }
}
