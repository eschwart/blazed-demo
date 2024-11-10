use crate::{clap::Parser, *};

#[derive(Parser, Debug)]
pub struct Config {
    /// TCP IP address
    #[arg(short, long)]
    tcp_addr: SocketAddr,

    /// UDP IP address
    #[arg(short, long)]
    udp_addr: SocketAddr,
}

impl Config {
    pub const fn tcp_addr(&self) -> SocketAddr {
        self.tcp_addr
    }

    pub const fn udp_addr(&self) -> SocketAddr {
        self.udp_addr
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::parse()
    }
}
