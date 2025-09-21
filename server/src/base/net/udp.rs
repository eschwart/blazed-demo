use crate::*;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::{net::SocketAddr, time::Duration};

fn game_handler(
    s: &SyncSelect,
    waiter_game: Waiter,
    clients_udp: UdpClients,
    updates: Updates,
) -> JoinHandle<Result> {
    s.spawn(move || {
        let spinner: SpinSleeper = Default::default();
        let setter_game = waiter_game.setter();

        loop {
            waiter_game.wait();

            // begin game updates
            loop {
                let mut is_idle = true;
                for (addr, client) in clients_udp.write().iter_mut() {
                    if !client.keys.is_empty() {
                        client.cam.input(client.keys);
                        updates.lock().get_mut(addr).unwrap().cam.eye = Some(client.cam.eye);
                        is_idle = false;
                    }
                }
                if is_idle {
                    setter_game.set_ready();
                    break;
                }

                // respect game speed
                spinner.sleep(GAME_SPEED);
            }
            waiter_game.reset();
        }
    })
}

fn handle_dist(
    s: &SyncSelect,
    waiter_dist: Waiter,
    spec_game: Spectator,
    udp: UdpServer,
    clients_udp: UdpClients,
    updates: Updates,
    tps: Duration,
) -> JoinHandle<Result> {
    s.spawn(move || -> Result {
        let spinner: SpinSleeper = Default::default();

        loop {
            // if idle, yield until a packet is received
            if !spec_game.is_ready() {
                waiter_dist.wait();
                waiter_dist.reset();
            }

            // TODO - improve this (try not to collect)
            let new_updates = updates
                .lock()
                .iter_mut()
                .filter_map(|(.., upt)| upt.is_modified().then_some(upt.take()))
                .collect::<Vec<UptObjOpt>>();

            // distribute updates to each player
            for upt in new_updates {
                // send update to each client
                for addr in clients_udp.read().keys() {
                    if let Err(e) = udp.send_to(&upt.serialize(), addr) {
                        error!("{e:?}")
                    }
                }
            }

            // respect the TPS
            spinner.sleep(tps);
        }
    })
}

/// control updates and handle status (parked/unparked) of TPS thread
fn handle_packets(
    s: &SyncSelect,
    waiter_game: Waiter,
    notifier_dist: Notifier,
    clients_udp: UdpClients,
    receiver: Receiver<(Packet, SocketAddr)>,
    updates: Updates,
) {
    /// process packet and return recipient's address
    fn _handle_packets(
        notifier: &Notifier,
        clients_udp: &UdpClients,
        receiver: &Receiver<(Packet, SocketAddr)>,
        updates: &Updates,
    ) -> Result<()> {
        // receive packet with address
        let (packet, addr) = receiver.recv()?;

        // prepare to update player data
        let mut clients = clients_udp.write();
        let obj = clients.get_mut(&addr).ok_or("Object no longer exists")?;

        // only processing input-based events for now
        match packet[0] {
            // Keys
            Keyboard::ID => {
                let kb = Keyboard::deserialize(&packet[1..]);
                if kb.is_pressed == 1 {
                    let was_empty = obj.keys.is_empty();
                    obj.keys |= Keys::from_bits_retain(kb.bits);
                    if was_empty {
                        notifier.notify();
                    }
                } else {
                    obj.keys -= Keys::from_bits_retain(kb.bits);
                }
            }

            // Wheel
            Wheel::ID => {
                let wheel = Wheel::deserialize(&packet[1..]);
                obj.cam.upt_fov(wheel.precise_y);
                updates.lock().get_mut(&addr).unwrap().cam.fov = Some(obj.cam.fov);
            }

            // Motion
            MotionOpt::ID => {
                let motion = MotionOpt::deserialize(&packet[1..]);
                obj.cam.look_at(
                    motion.xrel.unwrap_or_default(),
                    motion.yrel.unwrap_or_default(),
                );
                let mut lock = updates.lock();
                let client_cam = &mut lock.get_mut(&addr).unwrap().cam;
                client_cam.yaw = Some(obj.cam.yaw);
                client_cam.pitch = Some(obj.cam.pitch);
                client_cam.target = Some(obj.cam.target)
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    let notifier_game = waiter_game.notifier();

    game_handler(s, waiter_game, clients_udp.clone(), updates.clone());

    s.spawn(move || -> Result {
        loop {
            match _handle_packets(&notifier_game, &clients_udp, &receiver, &updates) {
                Ok(_) => {
                    // notify distribution thread
                    notifier_dist.notify();
                }
                Err(e) => error!("{e:?}"),
            }
        }
    });
}

/// instantiates TPS and packet processing threads
fn init_write(
    s: &SyncSelect,
    udp: UdpServer,
    clients_udp: UdpClients,
    receiver: Receiver<(Packet, SocketAddr)>,
    updates: Updates,
    tps: Duration,
) {
    let waiter_dist = Waiter::default();
    let notifier_dist = waiter_dist.notifier();

    let waiter_game = Waiter::default();
    let spectator_game = waiter_game.spectator();

    handle_dist(
        s,
        waiter_dist,
        spectator_game,
        udp,
        clients_udp.clone(),
        updates.clone(),
        tps,
    );

    handle_packets(
        s,
        waiter_game,
        notifier_dist,
        clients_udp,
        receiver,
        updates,
    );
}

/// UDP datagram message distributing thread
fn handle_incoming(
    s: &SyncSelect,
    udp: UdpServer,
    clients_udp: UdpClients,
    sender_packet: Sender<(Packet, SocketAddr)>,
    sender_addr: Sender<SocketAddr>,
) -> JoinHandle<Result> {
    s.spawn(move || {
        let mut buf = [0; PACKET_SIZE];

        loop {
            // receive datagram message from any client
            match udp.recv_from(&mut buf) {
                Ok((n, addr)) => {
                    let packet = buf[..n].to_vec();

                    // if the client exists,
                    // channel packet and source to process handling thread
                    if clients_udp.read().contains_key(&addr) {
                        // send to read channel
                        _ = sender_packet.try_send((packet, addr));
                        continue;
                    }

                    // if client doesn't exist, assume packet is client handshake
                    if packet[0] == ClientHandshake::ID {
                        debug!("[UDP] [4] Received client handshake");
                    } else {
                        error!("[UDP] [4] Expected PacketClientHandshake");
                        continue;
                    }

                    // share address with TCP server
                    debug!("[UDP] [5] Channeling UDP address");
                    if let Err(e) = sender_addr.send(addr) {
                        error!("{e:?}");
                        continue;
                    }
                }
                Err(e) => {
                    // debugging
                    error!("{e:?}");
                    break Ok(());
                }
            }
        }
    })
}

pub fn init_udp(
    s: &SyncSelect,
    udp: UdpServer,
    clients_udp: UdpClients,
    sender_addr: Sender<SocketAddr>,
    updates: Updates,
    tps: Duration,
) {
    // real-time game data channel
    let (sender_packet, receiver_packet) = bounded(8);

    // handle outgoing
    init_write(
        s,
        udp.clone(),
        clients_udp.clone(),
        receiver_packet,
        updates,
        tps,
    );

    // handle incoming UDP packets
    handle_incoming(s, udp.clone(), clients_udp, sender_packet, sender_addr);
}
