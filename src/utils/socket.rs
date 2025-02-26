use crate::utils::config::SharedPeer;
use once_cell::sync::Lazy;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
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

#[derive(Debug)]
pub enum SocketError {
    Io(IoError),
    Custom(String),
}

impl From<IoError> for SocketError {
    fn from(err: IoError) -> Self {
        SocketError::Io(err)
    }
}

pub enum ProtocolType {
    Udp,
    Tcp,
    #[cfg(feature = "bp")]
    Bp,
}

pub trait SendingSocket {
    fn new(address: &str) -> Result<Self, SocketError>
    where
        Self: Sized;
    fn send(&mut self, message: &str) -> Result<usize, SocketError>;
}

pub struct UdpSendingSocket {
    socket: Option<UdpSocket>,
    addr: SocketAddr,
}

impl UdpSendingSocket {
    async fn async_new(address: &str) -> Result<Self, SocketError> {
        let addr: SocketAddr = address
            .parse()
            .map_err(|e: std::net::AddrParseError| SocketError::Custom(e.to_string()))?;
        let local: SocketAddr = if addr.is_ipv4() {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        let std_socket = Socket::new(Domain::for_address(addr), Type::DGRAM, None)?;
        std_socket.bind(&SockAddr::from(local))?;
        std_socket.set_nonblocking(true)?;
        let udp_socket = UdpSocket::from_std(std_socket.into())?;
        Ok(Self {
            socket: Some(udp_socket),
            addr,
        })
    }

    async fn async_send(&mut self, message: &str) -> Result<usize, SocketError> {
        if let Some(ref mut sock) = self.socket {
            maybe_delay().await;
            let n = sock.send_to(message.as_bytes(), &self.addr).await?;
            Ok(n)
        } else {
            Err(SocketError::Custom("UDP socket missing".to_string()))
        }
    }
}

impl SendingSocket for UdpSendingSocket {
    fn new(address: &str) -> Result<Self, SocketError> {
        TOKIO_RUNTIME.block_on(async { Self::async_new(address).await })
    }
    fn send(&mut self, message: &str) -> Result<usize, SocketError> {
        TOKIO_RUNTIME.block_on(async { self.async_send(message).await })
    }
}

pub struct TcpSendingSocket {
    stream: Option<TcpStream>,
}

impl TcpSendingSocket {
    async fn async_new(address: &str) -> Result<Self, SocketError> {
        let addr: SocketAddr = address
            .parse()
            .map_err(|e: std::net::AddrParseError| SocketError::Custom(e.to_string()))?;
        let std_socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
        std_socket.connect(&SockAddr::from(addr))?;
        std_socket.set_nonblocking(true)?;
        let stream = TcpStream::from_std(std_socket.into())?;
        Ok(Self {
            stream: Some(stream),
        })
    }

    async fn async_send(&mut self, message: &str) -> Result<usize, SocketError> {
        if let Some(ref mut s) = self.stream {
            maybe_delay().await;
            s.write_all(message.as_bytes()).await?;
            s.shutdown().await?;
            Ok(message.len())
        } else {
            Err(SocketError::Custom("TCP stream missing".to_string()))
        }
    }
}

impl SendingSocket for TcpSendingSocket {
    fn new(address: &str) -> Result<Self, SocketError> {
        TOKIO_RUNTIME.block_on(async { Self::async_new(address).await })
    }
    fn send(&mut self, message: &str) -> Result<usize, SocketError> {
        TOKIO_RUNTIME.block_on(async { self.async_send(message).await })
    }
}

#[cfg(feature = "bp")]
mod bp_socket {
    use super::*;
    const AF_BP: i32 = 28;
    #[repr(C)]
    struct sockaddr_bp {
        sa_family: u16,
        sa_data: [u8; 14],
    }
    pub struct BpSendingSocket {
        socket: Option<UdpSocket>,
        addr: SocketAddr,
    }
    impl BpSendingSocket {
        async fn async_new(address: &str) -> Result<Self, SocketError> {
            let addr: SocketAddr = address
                .parse()
                .map_err(|e: std::net::AddrParseError| SocketError::Custom(e.to_string()))?;
            let local: SocketAddr = if addr.is_ipv4() {
                "0.0.0.0:0".parse().unwrap()
            } else {
                "[::]:0".parse().unwrap()
            };
            let std_socket = Socket::new_raw(Domain::from_raw(AF_BP), Type::DGRAM, None)?;
            std_socket.bind(&SockAddr::from(local))?;
            std_socket.set_nonblocking(true)?;
            let socket = UdpSocket::from_std(std_socket.into())?;
            Ok(Self {
                socket: Some(socket),
                addr,
            })
        }
        async fn async_send(&mut self, message: &str) -> Result<usize, SocketError> {
            if let Some(ref mut sock) = self.socket {
                maybe_delay().await;
                println!("(BP) Stub sending '{}' to '{}'", message, self.addr);
                Ok(message.len())
            } else {
                Err(SocketError::Custom("BP socket missing".to_string()))
            }
        }
    }
    impl SendingSocket for BpSendingSocket {
        fn new(address: &str) -> Result<Self, SocketError> {
            TOKIO_RUNTIME.block_on(async { Self::async_new(address).await })
        }
        fn send(&mut self, message: &str) -> Result<usize, SocketError> {
            TOKIO_RUNTIME.block_on(async { self.async_send(message).await })
        }
    }
}
#[cfg(feature = "bp")]
pub use bp_socket::BpSendingSocket;

pub fn create_sending_socket(
    protocol: ProtocolType,
    address: &str,
) -> Result<Box<dyn SendingSocket>, SocketError> {
    match protocol {
        ProtocolType::Udp => Ok(Box::new(UdpSendingSocket::new(address)?)),
        ProtocolType::Tcp => Ok(Box::new(TcpSendingSocket::new(address)?)),
        #[cfg(feature = "bp")]
        ProtocolType::Bp => Ok(Box::new(bp_socket::BpSendingSocket::new(address)?)),
    }
}

pub async fn start_udp_listener(address: &str) -> Result<UdpSocket, SocketError> {
    let addr: SocketAddr = address
        .parse()
        .map_err(|e: std::net::AddrParseError| SocketError::Custom(e.to_string()))?;
    let socket = UdpSocket::bind(addr).await?;
    Ok(socket)
}

pub async fn start_tcp_listener(address: &str) -> Result<TcpListener, SocketError> {
    let addr: SocketAddr = address
        .parse()
        .map_err(|e: std::net::AddrParseError| SocketError::Custom(e.to_string()))?;
    let listener = TcpListener::bind(addr).await?;
    Ok(listener)
}

pub trait SocketObserver: Send + Sync {
    fn on_socket_event(&self, text: &str, sender: SharedPeer);
}

pub trait SocketController: Send + Sync {
    fn add_observer(&mut self, observer: Arc<dyn SocketObserver + Send + Sync>);
}

pub struct DefaultSocketController {
    observers: Vec<Arc<dyn SocketObserver + Send + Sync>>,
    local_peer: Option<SharedPeer>,
    udp_socket: Option<UdpSocket>,
    tcp_listener: Option<TcpListener>,
}

impl DefaultSocketController {
    pub fn new() -> Self {
        Self {
            observers: Vec::new(),
            local_peer: None,
            udp_socket: None,
            tcp_listener: None,
        }
    }

    fn notify_observers(&self, text: &str, sender: SharedPeer) {
        for obs in &self.observers {
            obs.on_socket_event(text, sender.clone());
        }
    }

    async fn run_udp_listener(
        controller_arc: Arc<Mutex<DefaultSocketController>>,
        socket: UdpSocket,
    ) {
        let mut buf = [0u8; 1024];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((n, _)) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let sender = { controller_arc.lock().unwrap().local_peer.clone().unwrap() };
                    controller_arc
                        .lock()
                        .unwrap()
                        .notify_observers(&text, sender);
                }
                Err(_) => {
                    sleep(Duration::from_secs(1)).await;
                    continue;
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
                Ok((mut stream, _)) => {
                    let mut buf = [0u8; 1024];
                    if let Ok(n) = stream.read(&mut buf).await {
                        if n > 0 {
                            let text = String::from_utf8_lossy(&buf[..n]).to_string();
                            let sender =
                                { controller_arc.lock().unwrap().local_peer.clone().unwrap() };
                            controller_arc
                                .lock()
                                .unwrap()
                                .notify_observers(&text, sender);
                        }
                    }
                }
                Err(_) => {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }
        }
    }

    pub fn init_controller(
        local_peer: SharedPeer,
    ) -> Result<Arc<Mutex<DefaultSocketController>>, SocketError> {
        let udp_socket =
            TOKIO_RUNTIME.block_on(async { start_udp_listener("127.0.0.1:7000").await })?;
        let tcp_listener =
            TOKIO_RUNTIME.block_on(async { start_tcp_listener("127.0.0.1:7001").await })?;
        let mut controller = Self::new();
        controller.local_peer = Some(local_peer);
        controller.udp_socket = Some(udp_socket);
        controller.tcp_listener = Some(tcp_listener);
        let controller_arc: Arc<Mutex<DefaultSocketController>> = Arc::new(Mutex::new(controller));
        {
            let arc_clone = Arc::clone(&controller_arc);
            TOKIO_RUNTIME.spawn(async move {
                let socket = arc_clone.lock().unwrap().udp_socket.take().unwrap();
                DefaultSocketController::run_udp_listener(arc_clone, socket).await;
            });
        }
        {
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
