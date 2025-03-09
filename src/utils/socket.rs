use crate::utils::config::Peer;
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
        let addr: SocketAddr = address.parse()?;
        let local = ephemeral_local_addr_for(&addr);

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
        let socket = self
            .socket
            .as_mut()
            .ok_or_else(|| SocketError::Custom("UDP socket missing".to_string()))?;

        maybe_delay().await;
        println!(
            "UDP: sending {} bytes to {}, content: \"{}\"",
            message.len(),
            self.addr,
            message
        );

        let bytes_sent = socket.send_to(message.as_bytes(), &self.addr).await?;
        println!(
            "UDP: successfully sent {} bytes to {}",
            bytes_sent, self.addr
        );

        Ok(bytes_sent)
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
        let addr: SocketAddr = address.parse()?;
        let std_socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;

        std_socket.connect(&SockAddr::from(addr)).map_err(|e| {
            eprintln!("TCP: failed to connect to {}: {:?}", addr, e);
            SocketError::Io(e)
        })?;

        std_socket.set_nonblocking(true)?;
        let stream = TcpStream::from_std(std_socket.into())?;

        Ok(Self {
            stream: Some(stream),
        })
    }

    async fn async_send(&mut self, message: &str) -> Result<usize, SocketError> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| SocketError::Custom("TCP stream missing".to_string()))?;

        maybe_delay().await;
        println!(
            "TCP: sending {} bytes to remote, content: \"{}\"",
            message.len(),
            message
        );

        stream.write_all(message.as_bytes()).await?;
        stream.shutdown().await?;

        println!("TCP: successfully sent {} bytes to remote", message.len());
        Ok(message.len())
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
        pub async fn async_new(address: &str) -> Result<Self, SocketError> {
            let addr: SocketAddr = address.parse()?;
            let local = ephemeral_local_addr_for(&addr);

            let std_socket = Socket::new_raw(Domain::from_raw(AF_BP), Type::DGRAM, None)?;
            std_socket.bind(&SockAddr::from(local))?;
            std_socket.set_nonblocking(true)?;
            let socket = UdpSocket::from_std(std_socket.into())?;

            Ok(Self {
                socket: Some(socket),
                addr,
            })
        }

        pub async fn async_send(&mut self, message: &str) -> Result<usize, SocketError> {
            let socket = self
                .socket
                .as_mut()
                .ok_or_else(|| SocketError::Custom("BP socket missing".to_string()))?;

            maybe_delay().await;
            println!(
                "(BP) Stub sending '{}' ({} bytes) to '{}'",
                message,
                message.len(),
                self.addr
            );

            Ok(message.len())
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
        ProtocolType::Bp => Ok(Box::new(BpSendingSocket::new(address)?)),
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

    fn notify_observers(&self, text: &str, sender: Peer) {
        for observer in &self.observers {
            observer.on_socket_event(text.trim(), sender.clone());
        }
    }

    async fn run_udp_listener(
        controller_arc: Arc<Mutex<DefaultSocketController>>,
        socket: UdpSocket,
    ) {
        let mut buf = [0u8; 1024];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((n, addr)) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let sender = controller_arc.lock().unwrap().local_peer.clone().unwrap();
                    println!(
                        "UDP: received {} bytes from {}, content: \"{}\"",
                        n, addr, text
                    );
                    controller_arc
                        .lock()
                        .unwrap()
                        .notify_observers(&text, sender);
                }
                Err(e) => {
                    eprintln!("UDP: receiving error {:?}", e);
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
                    let mut buf = [0u8; 1024];
                    match stream.read(&mut buf).await {
                        Ok(n) if n > 0 => {
                            let text = String::from_utf8_lossy(&buf[..n]).to_string();
                            let sender = controller_arc.lock().unwrap().local_peer.clone().unwrap();
                            println!(
                                "TCP: received {} bytes from {}, content: \"{}\"",
                                n, addr, text
                            );
                            controller_arc
                                .lock()
                                .unwrap()
                                .notify_observers(&text, sender);
                        }
                        Ok(_) => {
                            println!("TCP: zero bytes read from {}, closing", addr);
                        }
                        Err(e) => {
                            eprintln!("TCP: read error {:?} from {}", e, addr);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("TCP: accept error {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    pub fn init_controller(
        local_peer: Peer,
    ) -> Result<Arc<Mutex<DefaultSocketController>>, SocketError> {
        let udp_socket =
            TOKIO_RUNTIME.block_on(async { start_udp_listener("127.0.0.1:7000").await })?;
        let tcp_listener =
            TOKIO_RUNTIME.block_on(async { start_tcp_listener("127.0.0.1:7001").await })?;

        let mut controller = Self::new();
        controller.local_peer = Some(local_peer);
        controller.udp_socket = Some(udp_socket);
        controller.tcp_listener = Some(tcp_listener);

        let controller_arc = Arc::new(Mutex::new(controller));

        let arc_clone = Arc::clone(&controller_arc);
        TOKIO_RUNTIME.spawn(async move {
            let socket = arc_clone.lock().unwrap().udp_socket.take().unwrap();
            DefaultSocketController::run_udp_listener(arc_clone, socket).await;
        });

        let arc_clone = Arc::clone(&controller_arc);
        TOKIO_RUNTIME.spawn(async move {
            let listener = arc_clone.lock().unwrap().tcp_listener.take().unwrap();
            DefaultSocketController::run_tcp_listener(arc_clone, listener).await;
        });

        Ok(controller_arc)
    }
}

impl SocketController for DefaultSocketController {
    fn add_observer(&mut self, observer: Arc<dyn SocketObserver + Send + Sync>) {
        self.observers.push(observer);
    }
}
