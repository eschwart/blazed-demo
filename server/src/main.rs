mod base;

use base::*;
use crossbeam_channel::unbounded;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{atomic::Ordering, Arc},
    thread::{spawn, JoinHandle},
};
use sync_select::*;

pub type TcpClients = Arc<RwLock<HashMap<Id, TcpClient>>>;
pub type UdpClients = Arc<RwLock<HashMap<SocketAddr, ObjectData>>>;
pub type Updates = Arc<Mutex<HashSet<SocketAddr>>>;

fn handle_ctrlc(s: &SyncSelect) -> Result {
    let thread = s.thread();
    ctrlc::set_handler(move || thread.unpark()).map_err(Into::into)
}

fn main() -> Result {
    init_logger();
    let cfg = Config::default();

    // init TCP server
    let tcp = TcpServer::new(cfg.tcp_addr())?;
    info!("TCP @ {:?}", cfg.tcp_addr());

    // init UDP server
    let udp = UdpServer::new(cfg.udp_addr())?;
    let udp_clone = udp.try_clone()?;
    info!("UDP @ {:?}", cfg.udp_addr());

    // map of player streams
    let clients_tcp: TcpClients = Default::default();

    // map of player data
    let clients_udp: UdpClients = Default::default();

    // share client UDP sockets between main thread and 'alive' thread
    let (sender_addr, receiver_addr) = unbounded::<SocketAddr>();

    // share client TCP packets
    let (sender_packet, receiver_packet) = unbounded::<Packet>();

    // monotonic user identity (default: 0)
    let id = Default::default();

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
        id,
    );

    // handle UDP packets
    init_udp(&s, udp, udp_clone, clients_udp, sender_addr, cfg.tps());

    Ok(())
}
