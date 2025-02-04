use once_cell::sync::Lazy;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::Error as IoError;
use std::net::SocketAddr;
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
            .parse::<SocketAddr>()
            .map_err(|e| SocketError::Custom(e.to_string()))?;
        let local: SocketAddr = if addr.is_ipv4() {
            "0.0.0.0:0".parse::<SocketAddr>().unwrap()
        } else {
            "[::]:0".parse::<SocketAddr>().unwrap()
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
            .parse::<SocketAddr>()
            .map_err(|e| SocketError::Custom(e.to_string()))?;
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

pub enum ProtocolType {
    Udp,
    Tcp,
    #[cfg(feature = "bp")]
    Bp,
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
                .parse::<SocketAddr>()
                .map_err(|e| SocketError::Custom(e.to_string()))?;
            let local: SocketAddr = if addr.is_ipv4() {
                "0.0.0.0:0".parse::<SocketAddr>().unwrap()
            } else {
                "[::]:0".parse::<SocketAddr>().unwrap()
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
        ProtocolType::Bp => Ok(Box::new(BpSendingSocket::new(address)?)),
    }
}

pub async fn start_udp_listener(address: &str) -> Result<UdpSocket, SocketError> {
    let addr: SocketAddr = address
        .parse::<SocketAddr>()
        .map_err(|e| SocketError::Custom(e.to_string()))?;
    let socket = UdpSocket::bind(addr).await?;
    Ok(socket)
}

pub async fn start_tcp_listener(address: &str) -> Result<TcpListener, SocketError> {
    let addr: SocketAddr = address
        .parse::<SocketAddr>()
        .map_err(|e| SocketError::Custom(e.to_string()))?;
    let listener = TcpListener::bind(addr).await?;
    Ok(listener)
}

pub async fn run_udp_listener(
    socket: UdpSocket,
    app: std::sync::Arc<std::sync::Mutex<crate::app::ChatApp>>,
    local_peer: crate::utils::config::SharedPeer,
) -> Result<(), SocketError> {
    let mut buf = [0u8; 1024];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((n, _addr)) => {
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let mut locked_app = app.lock().unwrap();
                crate::utils::message::Message::receive(
                    &mut locked_app,
                    &text,
                    std::sync::Arc::clone(&local_peer),
                );
            }
            Err(e) => {
                eprintln!("UDP recv error: {}", e);
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        }
    }
}

pub async fn run_tcp_listener(
    listener: TcpListener,
    app: std::sync::Arc<std::sync::Mutex<crate::app::ChatApp>>,
    local_peer: crate::utils::config::SharedPeer,
) -> Result<(), SocketError> {
    loop {
        match listener.accept().await {
            Ok((mut stream, _addr)) => {
                let mut buf = [0u8; 1024];
                match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => {
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        let mut locked_app = app.lock().unwrap();
                        crate::utils::message::Message::receive(
                            &mut locked_app,
                            &text,
                            std::sync::Arc::clone(&local_peer),
                        );
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        eprintln!("TCP read error: {}", e);
                        continue;
                    }
                }
            }
            Err(e) => {
                eprintln!("TCP accept error: {}", e);
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        }
    }
}
