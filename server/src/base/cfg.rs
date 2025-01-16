use crate::*;
use clap::{value_parser, Parser};
use std::time::Duration;

/// Calculates the duration of a single tick.
fn calc_tps(tps: u16) -> Duration {
    Duration::from_secs_f32((1000.0 / tps as f32) / 1000.0)
}

#[derive(Parser, Debug)]
pub struct Config {
    /// Local TCP IP address
    #[arg(short, long, default_value_t = get_socket_addr(TCP_PORT))]
    tcp_addr: SocketAddr,

    /// Local UDP IP address
    #[arg(short, long, default_value_t = get_socket_addr(UDP_PORT))]
    udp_addr: SocketAddr,

    /// Server ticks/sec
    #[arg(long, default_value_t = 128, value_parser = value_parser!(u16).range(1..1024))]
    tps: u16,
}

impl Config {
    pub const fn tcp_addr(&self) -> SocketAddr {
        self.tcp_addr
    }

    pub const fn udp_addr(&self) -> SocketAddr {
        self.udp_addr
    }

    pub fn tps(&self) -> Duration {
        calc_tps(self.tps)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::parse()
    }
}
