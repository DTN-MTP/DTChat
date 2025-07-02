use serde::{Deserialize, Serialize};
use socket2::SockAddr;
use std::{
    fmt,
    io::{self, Error, ErrorKind},
    mem::{self, ManuallyDrop},
    net::SocketAddr,
    ptr,
    str::FromStr,
};

const AF_BP: libc::c_int = 28;

/// Custom error types for network operations
#[derive(Debug)]
pub enum NetworkError {
    InvalidFormat(String),
    UnsupportedScheme(String),
    Io(io::Error),
    AddressParseError(String),
    BpNotImplemented(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::InvalidFormat(msg) => write!(f, "Invalid endpoint format: {}", msg),
            NetworkError::UnsupportedScheme(scheme) => write!(f, "Unsupported scheme: {}", scheme),
            NetworkError::Io(err) => write!(f, "IO error: {}", err),
            NetworkError::AddressParseError(msg) => write!(f, "Address parse error: {}", msg),
            NetworkError::BpNotImplemented(msg) => write!(f, "BP scheme not implemented: {}", msg),
        }
    }
}

impl std::error::Error for NetworkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NetworkError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for NetworkError {
    fn from(err: io::Error) -> Self {
        NetworkError::Io(err)
    }
}

/// Type alias for network results
pub type NetworkResult<T> = Result<T, NetworkError>;

/// Network endpoint supporting different protocols
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "address")]
pub enum Endpoint {
    Udp(String),
    Tcp(String),
    Bp(String),
}

impl Endpoint {
    /// Check if this endpoint is valid and can be used for socket operations
    pub fn is_valid(&self) -> bool {
        match self {
            Endpoint::Udp(addr) | Endpoint::Tcp(addr) => {
                addr.parse::<SocketAddr>().is_ok()
            }
            Endpoint::Bp(addr) => {
                !addr.contains("PLACEHOLDER")
                    && !addr.is_empty()
                    && (addr.starts_with("ipn:") || addr.starts_with("dtn:"))
            }
        }
    }

    /// Get the protocol type as a string
    pub fn protocol(&self) -> &'static str {
        match self {
            Endpoint::Udp(_) => "udp",
            Endpoint::Tcp(_) => "tcp",
            Endpoint::Bp(_) => "bp",
        }
    }

    /// Get the address part of the endpoint
    pub fn address(&self) -> &str {
        match self {
            Endpoint::Udp(addr) | Endpoint::Tcp(addr) | Endpoint::Bp(addr) => addr,
        }
    }

    /// Create an endpoint from protocol and address
    pub fn new(protocol: &str, address: impl Into<String>) -> NetworkResult<Self> {
        let address = address.into();
        match protocol.to_lowercase().as_str() {
            "udp" => Ok(Endpoint::Udp(address)),
            "tcp" => Ok(Endpoint::Tcp(address)),
            "bp" => Ok(Endpoint::Bp(address)),
            _ => Err(NetworkError::UnsupportedScheme(protocol.to_string())),
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Endpoint::Udp(addr) => write!(f, "udp://{}", addr),
            Endpoint::Tcp(addr) => write!(f, "tcp://{}", addr),
            Endpoint::Bp(addr) => write!(f, "bp://{}", addr),
        }
    }
}

impl FromStr for Endpoint {
    type Err = NetworkError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Handle both "protocol://address" and "protocol address" formats
        if let Some(captures) = s.split_once("://") {
            let (protocol, address) = captures;
            Self::new(protocol, address)
        } else if let Some(captures) = s.split_once(' ') {
            let (protocol, address) = captures;
            Self::new(protocol, address)
        } else {
            Err(NetworkError::InvalidFormat(s.to_string()))
        }
    }
}

const BP_SCHEME_IPN: u32 = 1;
const BP_SCHEME_DTN: u32 = 2;

#[repr(C)]
struct SockAddrBp {
    bp_family: libc::sa_family_t,
    bp_scheme: u32,
    bp_addr: BpAddr,
}

#[repr(C)]
union BpAddr {
    ipn: ManuallyDrop<IpnAddr>,
    // Extend with other schemes like DTN if needed
}

#[repr(C)]
struct IpnAddr {
    node_id: u32,
    service_id: u32,
}

/// Convert a Bundle Protocol EID string to a socket address
pub fn create_bp_sockaddr(eid_string: &str) -> NetworkResult<SockAddr> {
    if eid_string.is_empty() {
        return Err(NetworkError::InvalidFormat("EID string cannot be empty".to_string()));
    }

    // Handle "ipn:" scheme
    if let Some(eid_body) = eid_string.strip_prefix("ipn:") {
        parse_ipn_address(eid_body)
    }
    // Handle unsupported schemes
    else if eid_string.starts_with("dtn:") {
        Err(NetworkError::BpNotImplemented("DTN scheme not yet implemented".to_string()))
    } else {
        Err(NetworkError::InvalidFormat(format!("Unsupported scheme in EID: {}", eid_string)))
    }
}

fn parse_ipn_address(eid_body: &str) -> NetworkResult<SockAddr> {
    let parts: Vec<&str> = eid_body.split('.').collect();
    if parts.len() != 2 {
        return Err(NetworkError::InvalidFormat(format!("Invalid IPN EID format: ipn:{}", eid_body)));
    }

    let node_id: u32 = parts[0]
        .parse()
        .map_err(|_| NetworkError::InvalidFormat("Invalid node ID".to_string()))?;
    let service_id: u32 = parts[1]
        .parse()
        .map_err(|_| NetworkError::InvalidFormat("Invalid service ID".to_string()))?;

    create_ipn_sockaddr(node_id, service_id)
}

fn create_ipn_sockaddr(node_id: u32, service_id: u32) -> NetworkResult<SockAddr> {
    let sockaddr_bp = SockAddrBp {
        bp_family: AF_BP as libc::sa_family_t,
        bp_scheme: BP_SCHEME_IPN,
        bp_addr: BpAddr {
            ipn: ManuallyDrop::new(IpnAddr {
                node_id,
                service_id,
            }),
        },
    };

    let mut sockaddr_storage: libc::sockaddr_storage = unsafe { mem::zeroed() };
    unsafe {
        ptr::copy_nonoverlapping(
            &sockaddr_bp as *const SockAddrBp as *const u8,
            &mut sockaddr_storage as *mut _ as *mut u8,
            mem::size_of::<SockAddrBp>(),
        );
    }

    let addr_len = mem::size_of::<SockAddrBp>() as libc::socklen_t;
    let address = unsafe { SockAddr::new(sockaddr_storage, addr_len) };
    Ok(address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_creation() {
        let tcp_endpoint = Endpoint::new("tcp", "127.0.0.1:8080").unwrap();
        assert_eq!(tcp_endpoint.protocol(), "tcp");
        assert_eq!(tcp_endpoint.address(), "127.0.0.1:8080");
    }

    #[test]
    fn test_endpoint_from_str() {
        let endpoint: Endpoint = "tcp://127.0.0.1:8080".parse().unwrap();
        assert_eq!(endpoint, Endpoint::Tcp("127.0.0.1:8080".to_string()));
    }

    #[test]
    fn test_endpoint_display() {
        let endpoint = Endpoint::Tcp("127.0.0.1:8080".to_string());
        assert_eq!(endpoint.to_string(), "tcp://127.0.0.1:8080");
    }

    #[test]
    fn test_endpoint_validation() {
        let valid_tcp = Endpoint::Tcp("127.0.0.1:8080".to_string());
        assert!(valid_tcp.is_valid());

        let invalid_tcp = Endpoint::Tcp("invalid_address".to_string());
        assert!(!invalid_tcp.is_valid());

        let valid_bp = Endpoint::Bp("ipn:1.1".to_string());
        assert!(valid_bp.is_valid());
    }
}
