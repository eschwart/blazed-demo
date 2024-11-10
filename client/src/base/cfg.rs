use crate::{clap::Parser, *};

use std::net::SocketAddr;

#[derive(Parser, Clone, Copy, Debug)]
pub struct ServerConfig {
    /// Remote TCP IP address
    #[arg(alias = "rt", long)]
    remote_tcp_addr: SocketAddr,

    /// Local IP address
    #[arg(alias = "lu", long)]
    local_udp_addr: SocketAddr,

    /// Remote IP address
    #[arg(alias = "ru", long)]
    remote_udp_addr: SocketAddr,
}

impl ServerConfig {
    pub const fn remote_tcp_addr(&self) -> SocketAddr {
        self.remote_tcp_addr
    }

    pub const fn local_udp_addr(&self) -> SocketAddr {
        self.local_udp_addr
    }

    pub const fn remote_udp_addr(&self) -> SocketAddr {
        self.remote_udp_addr
    }
}

#[derive(Parser, Debug)]
pub struct Config {
    #[command(subcommand)]
    cmd: Option<Subcommand>,
}

impl Config {
    pub const fn server(&self) -> Option<ServerConfig> {
        if let Some(Subcommand::Server(cfg)) = self.cmd {
            Some(cfg)
        } else {
            None
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::parse()
    }
}

#[derive(Parser, Debug)]
enum Subcommand {
    Server(ServerConfig),
}
