use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::SocketAddr;
use std::str::FromStr;
use std::mem::MaybeUninit;

pub enum AddressFamily {
    IPv4,
    IPv6,
    BP(i32),  
}

impl AddressFamily {
    fn to_domain(&self) -> Domain {
        match self {
            AddressFamily::IPv4 => Domain::IPV4,
            AddressFamily::IPv6 => Domain::IPV6,
            AddressFamily::BP(af) => Domain::from(*af), 
        }
    }
}

#[derive(Debug)]
pub struct ProtoMessage {
    msg_uuid: String,
    rx_time: String,
}

pub struct ChatSocket {
    socket: Socket,
}

impl ChatSocket {
    pub fn new(family: AddressFamily) -> io::Result<Self> {
        let socket = Socket::new(
            family.to_domain(),
            Type::STREAM,
            Some(Protocol::TCP),
        )?;
        
        socket.set_nonblocking(true)?;
        socket.set_keepalive(true)?;
        
        Ok(Self { socket })
    }

    pub fn connect(&mut self) -> io::Result<()> {
        #[cfg(feature = "add_delay")]
        std::thread::sleep(std::time::Duration::from_millis(1000));

        let addr = SocketAddr::from_str("127.0.0.1:8080")
            .expect("Invalid address");
        
        match self.socket.connect(&addr.into()) {
            Ok(_) => {
                println!("Connected to ncat server");
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("Connection in progress...");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn send(&self, data: &[u8]) -> io::Result<usize> {
        #[cfg(feature = "add_delay")]
        std::thread::sleep(std::time::Duration::from_millis(100));

        let mut message = Vec::from(data);
        message.push(b'\n');
        
        match self.socket.send(&message) {
            Ok(size) => {
                println!("Sent {} bytes", size);
                Ok(size)
            }
            Err(e) => {
                eprintln!("Send error: {}", e);
                Err(e)
            }
        }
    }
    //read response from ncat server
    pub fn receive(&self) -> io::Result<Option<String>> {
        let mut buffer = [MaybeUninit::uninit(); 1024];
        match self.socket.recv(&mut buffer) {
            Ok(size) => {
                if size > 0 {
                    let mut data = Vec::with_capacity(size);
                    for i in 0..size {
                        data.push(unsafe { buffer[i].assume_init() });
                    }
                    if let Ok(message) = String::from_utf8(data) {
                        return Ok(Some(message));
                    }
                }
                Ok(None)
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }
}