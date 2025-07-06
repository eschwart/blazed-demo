use crate::*;
use clap::Parser;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
pub struct Config {
    /// Specify the FPS.
    #[arg(long, default_value_t = 120)]
    fps: Id,

    /// Do not attempt to connect to server.
    #[arg(long, default_value_t)]
    offline: bool,

    /// Remote TCP IP address
    #[arg(alias = "rt", long, default_value_t = get_socket_addr(TCP_PORT))]
    remote_tcp_addr: SocketAddr,

    /// Local UDP IP address (optional)
    #[arg(alias = "lu", long)]
    local_udp_addr: Option<SocketAddr>,

    /// Remote UDP IP address
    #[arg(alias = "ru", long, default_value_t = get_socket_addr(UDP_PORT))]
    remote_udp_addr: SocketAddr,
}

impl Config {
    pub const fn fps(&self) -> Id {
        self.fps
    }

    pub const fn is_online(&self) -> bool {
        !self.offline
    }

    pub const fn remote_tcp_addr(&self) -> SocketAddr {
        self.remote_tcp_addr
    }

    pub const fn local_udp_addr(&self) -> Option<SocketAddr> {
        self.local_udp_addr
    }

    pub const fn remote_udp_addr(&self) -> SocketAddr {
        self.remote_udp_addr
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::parse()
    }
}
