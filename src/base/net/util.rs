use std::net::{SocketAddr, ToSocketAddrs};

/// Retrieve a default socket with specified port number.
pub fn get_socket_addr(port: u16) -> SocketAddr {
    ("127.0.0.1", port)
        .to_socket_addrs()
        .expect("Failed to retrieve socket address(s)")
        .next()
        .expect("No available socket address(s)")
}
