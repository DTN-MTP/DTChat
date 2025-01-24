use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::io::{Error as IoError, Write};
use std::net::{Shutdown, SocketAddr};
use std::time::Duration;

#[cfg(feature = "add_delay")]
fn maybe_delay() {
    std::thread::sleep(Duration::from_secs(1));
}

#[cfg(not(feature = "add_delay"))]
fn maybe_delay() {}

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
    socket: Socket,
    addr: SockAddr,
}

impl SendingSocket for UdpSendingSocket {
    fn new(address: &str) -> Result<Self, SocketError> {
        let addr: SocketAddr = address
            .parse::<SocketAddr>()
            .map_err(|e| SocketError::Custom(e.to_string()))?;
        let socket = Socket::new(Domain::for_address(addr), Type::DGRAM, None)?;
        let addr = SockAddr::from(addr);
        Ok(Self { socket, addr })
    }

    fn send(&mut self, message: &str) -> Result<usize, SocketError> {
        maybe_delay();
        let bytes_sent = self.socket.send_to(message.as_bytes(), &self.addr)?;
        std::thread::sleep(Duration::from_secs(1));
        Ok(bytes_sent)
    }
}

pub struct TcpSendingSocket {
    socket: Socket,
}

impl SendingSocket for TcpSendingSocket {
    fn new(address: &str) -> Result<Self, SocketError> {
        let addr: SocketAddr = address
            .parse::<SocketAddr>()
            .map_err(|e| SocketError::Custom(e.to_string()))?;
        let socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
        socket.connect(&SockAddr::from(addr))?;
        Ok(Self { socket })
    }

    fn send(&mut self, message: &str) -> Result<usize, SocketError> {
        maybe_delay();
        self.socket.write_all(message.as_bytes())?;
        self.socket.shutdown(Shutdown::Both)?;
        Ok(message.len())
    }
}

pub enum ProtocolType {
    Udp,
    Tcp,
    //Bp,
}

#[cfg(feature = "bp")]
mod bp_socket {
    use super::{maybe_delay, SendingSocket, Socket, SocketError};
    use socket2::{Domain, Type};
    use std::{mem, os::raw::c_ushort};

    const AF_BP: i32 = 28;

    #[repr(C)]
    struct sockaddr_bp {
        sa_family: c_ushort,
        sa_data: [u8; 14],
    }

    pub struct BpSendingSocket {
        socket: Socket,
        bp_address: String,
    }

    impl SendingSocket for BpSendingSocket {
        fn new(address: &str) -> Result<Self, SocketError> {
            let socket = Socket::new_raw(Domain::from_raw(AF_BP), Type::DGRAM, None)?;
            Ok(Self {
                socket,
                bp_address: address.to_owned(),
            })
        }

        fn send(&mut self, message: &str) -> Result<usize, SocketError> {
            maybe_delay();
            println!("(BP) Stub sending '{}' to '{}'", message, self.bp_address);
            Ok(message.len())
        }
    }
}

pub fn create_sending_socket(
    protocol: ProtocolType,
    address: &str,
) -> Result<Box<dyn SendingSocket>, SocketError> {
    match protocol {
        ProtocolType::Udp => Ok(Box::new(UdpSendingSocket::new(address)?)),
        ProtocolType::Tcp => Ok(Box::new(TcpSendingSocket::new(address)?)),
        //ProtocolType::Bp => Ok(Box::new(BpSendingSocket::new(address)?)),
    }
}
