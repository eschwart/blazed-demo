mod base;

use base::*;
use crossbeam_channel::unbounded;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, atomic::Ordering},
    thread::{JoinHandle, spawn},
};
use sync_select::*;

pub type TcpClients = Arc<RwLock<HashMap<Id, TcpClient>>>;
pub type UdpClients = Arc<RwLock<HashMap<SocketAddr, UptObj>>>;
pub type Updates = Arc<Mutex<HashMap<SocketAddr, UptObjOpt>>>;

fn handle_ctrlc(s: &SyncSelect) -> Result {
    let thread = s.thread();
    ctrlc::set_handler(move || thread.unpark()).map_err(Into::into)
}

pub type Packet = Vec<u8>;

fn main() -> Result {
    env_logger::init();
    let cfg = Config::default();

    // init TCP server
    let tcp = TcpServer::new(cfg.tcp_addr())?;
    info!("[TCP] Binded @ {:?}", cfg.tcp_addr());

    // init UDP server
    let udp = UdpServer::new(cfg.udp_addr())?;
    info!("[UDP] Binded @ {:?}", cfg.udp_addr());

    // map of player streams
    let clients_tcp: TcpClients = Default::default();

    // map of player data
    let clients_udp: UdpClients = Default::default();

    // share client UDP sockets between main thread and 'alive' thread
    let (sender_addr, receiver_addr) = unbounded::<SocketAddr>();

    // share client TCP packets
    let (sender_packet, receiver_packet) = unbounded::<Vec<u8>>();

    // monotonic user identity (default: 0)
    let id = Default::default();

    let updates: Updates = Default::default();

    // short-circuiting local thread manager
    let s = SyncSelect::default();

    // handle SIGINT
    handle_ctrlc(&s)?;

    // handle TCP packets
    init_tcp(
        &s,
        tcp,
        clients_tcp,
        clients_udp.clone(),
        sender_packet,
        receiver_addr,
        receiver_packet,
        updates.clone(),
        id,
    );

    // handle UDP packets
    init_udp(&s, udp, clients_udp, sender_addr, updates, cfg.tps());

    Ok(())
}
