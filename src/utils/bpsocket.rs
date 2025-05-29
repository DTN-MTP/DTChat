#![cfg(feature = "bp")]

use std::io::{self, Error, ErrorKind};
use std::os::unix::io::RawFd;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::str::FromStr;

use super::socket::{
    Endpoint, SendingSocket, DefaultSocketController, TOKIO_RUNTIME
};
use super::message::ChatMessage;
use super::proto::{serialize_message, deserialize_message};

const AF_BP: i32 = 28;

// Define the proper sockaddr_bp structure to match the kernel module
#[repr(C)]
struct SockaddrBp {
    bp_family: libc::sa_family_t,
    bp_agent_id: u8,
}

#[derive(Debug)]
pub struct BpSocket {
    fd: RawFd,
    endpoint: Endpoint,
    local_agent_id: u8,
    listening: bool,
}

pub struct BpAddress {
    ipn_node: u64,
    service_id: u8,
}

impl FromStr for BpAddress {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("ipn:") {
            return Err(Error::new(ErrorKind::InvalidInput, "BP address must start with 'ipn:'"));
        }
        
        let parts = s[4..].split('.').collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err(Error::new(ErrorKind::InvalidInput, "BP address must be in format 'ipn:node.service'"));
        }
        
        let node = parts[0].parse::<u64>().map_err(|_| {
            Error::new(ErrorKind::InvalidInput, "Invalid node number in BP address")
        })?;
        
        let service = parts[1].parse::<u8>().map_err(|_| {
            Error::new(ErrorKind::InvalidInput, "Invalid service number in BP address")
        })?;
        
        Ok(BpAddress {
            ipn_node: node,
            service_id: service,
        })
    }
}

impl BpSocket {
    pub fn new(endpoint: &Endpoint) -> Result<Self, Box<dyn std::error::Error>> {
        let address = match endpoint {
            Endpoint::Bp(addr) => addr,
            _ => return Err(Box::new(Error::new(ErrorKind::InvalidInput, "Not a BP endpoint"))),
        };
        
        // Parse "ipn:node.service" format to extract the local service ID
        let bp_addr = BpAddress::from_str(address)?;
        
        let fd = unsafe {
            libc::socket(AF_BP, libc::SOCK_DGRAM, 0)
        };
        
        if fd < 0 {
            return Err(Box::new(Error::new(ErrorKind::Other, "Failed to create BP socket")));
        }
        
        Ok(Self {
            fd,
            endpoint: endpoint.clone(),
            local_agent_id: bp_addr.service_id,
            listening: false,
        })
    }

    pub fn bind(&mut self) -> io::Result<()> {
        let addr = SockaddrBp {
            bp_family: AF_BP as libc::sa_family_t,
            bp_agent_id: self.local_agent_id,
        };
        
        let bind_result = unsafe {
            libc::bind(
                self.fd,
                &addr as *const SockaddrBp as *const libc::sockaddr,
                std::mem::size_of::<SockaddrBp>() as libc::socklen_t,
            )
        };
        
        if bind_result < 0 {
            return Err(Error::last_os_error());
        }
        
        Ok(())
    }
    
    pub fn send(&mut self, data: &[u8], dest_addr: &str) -> io::Result<usize> {
        if !dest_addr.starts_with("ipn:") {
            return Err(Error::new(ErrorKind::InvalidInput, "Invalid BP destination address"));
        }
        
        // Parse the destination address to get the agent_id
        let dest_bp_addr = BpAddress::from_str(dest_addr)
            .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("Invalid destination address: {}", e)))?;
        
        let addr = SockaddrBp {
            bp_family: AF_BP as libc::sa_family_t,
            bp_agent_id: dest_bp_addr.service_id,
        };
        
        let sent_size = unsafe {
            libc::sendto(
                self.fd,
                data.as_ptr() as *const libc::c_void,
                data.len(),
                0,
                &addr as *const SockaddrBp as *const libc::sockaddr,
                std::mem::size_of::<SockaddrBp>() as libc::socklen_t,
            )
        };
        
        if sent_size < 0 {
            return Err(Error::last_os_error());
        }
        
        Ok(sent_size as usize)
    }

    pub fn start_listener(
        &mut self,
        controller_arc: Arc<Mutex<DefaultSocketController>>,
    ) -> io::Result<()> {
        if self.listening {
            return Ok(());
        }
        
        self.listening = true;
        self.bind()?;
        
        let fd = self.fd;
        let endpoint_str = match &self.endpoint {
            Endpoint::Bp(addr) => addr.clone(),
            _ => unreachable!(),
        };
        
        TOKIO_RUNTIME.spawn_blocking(move || {
            loop {
                let mut buffer = [0u8; 1024];
                let read_size = unsafe {
                    libc::recvfrom(
                        fd,
                        buffer.as_mut_ptr() as *mut libc::c_void,
                        buffer.len(),
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                
                if read_size > 0 {
                    println!("BP received data on listening address {}", endpoint_str);
                    
                    // Clone the data to a new Vec to avoid borrowing buffer
                    let data_vec = buffer[..read_size as usize].to_vec();
                    let new_controller_arc = Arc::clone(&controller_arc);
                    
                    TOKIO_RUNTIME.spawn(async move {
                        let controller = new_controller_arc.lock().unwrap();
                        let peers = controller.get_peers();
                        
                        if let Some(message) = deserialize_message(&data_vec, &peers) {
                            controller.notify_observers(message);
                        }
                    });
                } else if read_size < 0 {
                    let err = Error::last_os_error();
                    if err.kind() == ErrorKind::WouldBlock {
                        thread::sleep(Duration::from_millis(10));
                    } else {
                        eprintln!("BP Socket Error: {}", err);
                        break;
                    }
                } else {
                    // read_size == 0, connection closed
                    break;
                }
            }
        });
        
        Ok(())
    }
}

impl Drop for BpSocket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

impl SendingSocket for BpSocket {
    fn send_message(&mut self, message: &ChatMessage) -> Result<usize, Box<dyn std::error::Error>> {
        let serialized = serialize_message(message);
        
        // Use the socket's endpoint (which is the destination) for sending
        let bp_endpoint = match &self.endpoint {
            Endpoint::Bp(addr) => addr.clone(),
            _ => return Err(Box::new(Error::new(ErrorKind::InvalidInput, "Socket endpoint is not BP"))),
        };
        
        let sent_size = self.send(&serialized, &bp_endpoint)?;
        Ok(sent_size)
    }
}

pub fn create_bp_socket(endpoint: &Endpoint) -> Result<Box<dyn SendingSocket>, Box<dyn std::error::Error>> {
    let bp_socket = BpSocket::new(endpoint)?;
    Ok(Box::new(bp_socket))
} 