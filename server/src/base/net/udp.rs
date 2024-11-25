use crate::*;

use std::{
    collections::HashSet,
    net::SocketAddr,
    thread::{park, Thread},
    time::Duration,
};

use crossbeam_channel::{bounded, Receiver, Sender};

fn handle_dist(
    s: &SyncSelect,
    udp: UdpServer,
    clients_udp: UdpClients,
    updated: Arc<Mutex<HashSet<SocketAddr>>>,
    advance: Running,
    tps: Duration,
) -> JoinHandle<Result> {
    s.spawn(move || -> Result {
        let (spinner, backoff): (SpinSleeper, Backoff) = Default::default();

        loop {
            spinner.sleep(tps);
            while !advance.load(Ordering::SeqCst) {
                if backoff.is_completed() {
                    park();
                } else {
                    backoff.snooze();
                }
            }

            // distribute updates to each player
            for player_addr in updated.lock().drain() {
                let player = match clients_udp.read().get(&player_addr) {
                    Some(&player) => player,
                    None => continue,
                };

                for addr in clients_udp.read().keys() {
                    if let Err(e) = udp.send_to(&Packet::Player(player), addr) {
                        error!("{:?}", e)
                    }
                }
            }
            backoff.reset();
            advance.store(false, Ordering::Release);
        }
    })
}

fn _handle_packets(
    clients_udp: &UdpClients,
    receiver: &Receiver<(Packet, SocketAddr)>,
) -> Result<SocketAddr> {
    let (packet, addr) = receiver.recv()?;

    let input = packet.into_input()?;

    let mut clients = clients_udp.write();
    let player = clients.get_mut(&addr).ok_or("Player no longer exists")?;

    match input {
        Input::Mouse { xrel, yrel } => player.attr_mut().look_at(xrel, yrel),
        Input::Keyboard { keys } => player.attr_mut().input(keys),
    };
    Ok(addr)
}

fn handle_packets(
    s: &SyncSelect,
    clients_udp: UdpClients,
    receiver: Receiver<(Packet, SocketAddr)>,
    updated: Arc<Mutex<HashSet<SocketAddr>>>,
    advance: Running,
    dist_thread: Thread,
) {
    s.spawn(move || -> Result {
        loop {
            match _handle_packets(&clients_udp, &receiver) {
                Ok(addr) => {
                    dist_thread.unpark();
                    updated.lock().insert(addr);
                    advance.store(true, Ordering::Release);
                }
                Err(e) => error!("{:?}", e),
            }
        }
    });
}

fn init_write(
    s: &SyncSelect,
    udp: UdpServer,
    clients_udp: UdpClients,
    receiver_packet: Receiver<(Packet, SocketAddr)>,
    tps: Duration,
) {
    let updated: Arc<Mutex<HashSet<SocketAddr>>> = Default::default();
    let advance: Running = Default::default();

    let dist_thread = handle_dist(
        s,
        udp,
        clients_udp.clone(),
        updated.clone(),
        advance.clone(),
        tps,
    );

    handle_packets(
        s,
        clients_udp,
        receiver_packet,
        updated,
        advance,
        dist_thread.thread().clone(),
    );
}

fn handle_incoming(
    s: &SyncSelect,
    udp: UdpServer,
    clients_udp: UdpClients,
    sender_packet: Sender<(Packet, SocketAddr)>,
    sender_addr: Sender<SocketAddr>,
) {
    s.spawn(move || -> Result {
        let mut buf = [0; PACKET_SIZE];

        loop {
            match udp.recv_from(&mut buf, PacketKind::all()) {
                Ok((packet, addr)) => {
                    // check if user already exists
                    if clients_udp.read().contains_key(&addr) {
                        // send to read channel
                        _ = sender_packet.try_send((packet, addr));
                        continue;
                    }

                    debug!("UDP [ ][3] Received handshake");

                    // validate packet
                    if let Err(e) = packet.into_client_handshake() {
                        error!("{:?}", e);
                        continue;
                    }

                    debug!("UDP [ ][4] Transferring address");

                    // share address with TCP server
                    if let Err(e) = sender_addr.send(addr) {
                        error!("{:?}", e);
                    }
                }
                Err(e) => {
                    error!("{:?}", e);
                    break Ok(());
                }
            }
        }
    });
}

pub fn init_udp(
    s: &SyncSelect,
    udp_a: UdpServer,
    udp_b: UdpServer,
    clients_udp: UdpClients,
    sender_addr: Sender<SocketAddr>,
    tps: Duration,
) {
    // real-time game data channel
    let (sender_packet, receiver_packet) = bounded(8);

    s.spawn_with(move |s| -> Result {
        // handle outgoing
        init_write(s, udp_a, clients_udp.clone(), receiver_packet, tps);

        // handle incoming UDP packets
        handle_incoming(s, udp_b, clients_udp, sender_packet, sender_addr);

        Ok(())
    });
}
