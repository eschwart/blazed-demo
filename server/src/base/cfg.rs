use std::time::Duration;

use clap::value_parser;

use crate::{clap::Parser, *};

/// Calculates the duration of a single tick.
fn calc_tps(tps: u8) -> Duration {
    Duration::from_secs_f32((1000.0 / tps as f32) / 1000.0)
}

#[derive(Parser, Debug)]
pub struct Config {
    /// TCP IP address
    #[arg(short, long)]
    tcp_addr: SocketAddr,

    /// UDP IP address
    #[arg(short, long)]
    udp_addr: SocketAddr,

    #[arg(long, default_value_t = 128, value_parser = value_parser!(u8).range(1..))]
    tps: u8,
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
