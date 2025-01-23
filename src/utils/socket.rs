use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::SocketAddr;
use std::str::FromStr;
use std::mem::MaybeUninit;

pub struct ChatSocket {
    socket: Socket,
}

impl ChatSocket {
    pub fn new() -> io::Result<Self> {
        let socket = Socket::new(
            Domain::IPV4,
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
    //maybe read response from ncat?
    pub fn receive(&self) -> io::Result<Vec<u8>> {
        let mut buffer = [MaybeUninit::uninit(); 1024];
        match self.socket.recv(&mut buffer) {
            Ok(size) => {
                let mut data = Vec::with_capacity(size);
                for i in 0..size {
                    data.push(unsafe { buffer[i].assume_init() });
                }
                Ok(data)
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                Ok(Vec::new())
            }
            Err(e) => Err(e),
        }
    }
}