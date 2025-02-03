use once_cell::sync::Lazy;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::Error as IoError;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, UdpSocket};
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
        let local_addr: SocketAddr = if addr.is_ipv4() {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        let std_socket = Socket::new(Domain::for_address(addr), Type::DGRAM, None)?;
        std_socket.bind(&SockAddr::from(local_addr))?;
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
            let bytes_sent = sock.send_to(message.as_bytes(), &self.addr).await?;
            sleep(Duration::from_secs(1)).await;
            Ok(bytes_sent)
        } else {
            Err(SocketError::Custom("UDP socket missing".to_string()))
        }
    }
}

impl SendingSocket for UdpSendingSocket {
    fn new(address: &str) -> Result<Self, SocketError> {
        TOKIO_RUNTIME.block_on(async { UdpSendingSocket::async_new(address).await })
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
        TOKIO_RUNTIME.block_on(async { TcpSendingSocket::async_new(address).await })
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
    pub struct BpSendingSocket {
        socket: Option<UdpSocket>,
        addr: SocketAddr,
    }
    impl BpSendingSocket {
        async fn async_new(address: &str) -> Result<Self, SocketError> {
            let addr: SocketAddr = address
                .parse::<SocketAddr>()
                .map_err(|e| SocketError::Custom(e.to_string()))?;
            let local_addr: SocketAddr = if addr.is_ipv4() {
                "0.0.0.0:0".parse().unwrap()
            } else {
                "[::]:0".parse().unwrap()
            };
            let std_socket = Socket::new(Domain::for_address(addr), Type::DGRAM, None)?;
            std_socket.bind(&SockAddr::from(local_addr))?;
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
            TOKIO_RUNTIME.block_on(async { BpSendingSocket::async_new(address).await })
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
