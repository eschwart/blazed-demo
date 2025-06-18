use crate::*;
use clap::Parser;
use std::{num::ParseIntError, time::Duration};

/// Calculates the duration of a single tick.
fn parse_tps(s: &str) -> Result<Duration, ParseIntError> {
    s.parse::<u64>()
        .map(|tps| Duration::from_secs_f32(1.0 / tps as f32))
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
    #[arg(long, default_value = "128", value_parser = parse_tps)]
    tps: Duration,
}

impl Config {
    pub const fn tcp_addr(&self) -> SocketAddr {
        self.tcp_addr
    }

    pub const fn udp_addr(&self) -> SocketAddr {
        self.udp_addr
    }

    pub const fn tps(&self) -> Duration {
        self.tps
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::parse()
    }
}
