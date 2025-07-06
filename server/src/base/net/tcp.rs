use crate::*;
use crossbeam_channel::{Receiver, Sender};
use std::time::Instant;
use ultraviolet::Vec3;

fn handshake(
    tcp: TcpClient,
    clients_udp: UdpClients,
    receiver_addr: &Receiver<SocketAddr>,
    id: Id,
) -> Result<SocketAddr> {
    let mut buf = [0; 1];

    // receive initial client handshake packet
    debug!("[TCP] [1] Receiving client handshake");
    tcp.recv(&mut buf)?;
    if buf[0] != ClientHandshake::ID {
        return Err(format!("Found {}, expected ClientHandshake.", buf[0]).into());
    }

    // reply with server handshake
    debug!("[TCP] [2] Sending server handshake");
    tcp.send(&ServerHandshake::new(id).serialize())?;

    // receive UDP address from UDP thread [handle_incoming]
    debug!("[TCP] [3] Waiting for UDP address");
    let addr = receiver_addr.recv()?;

    // send game states to client
    debug!("[TCP] [6] Sending game states");
    for &data in clients_udp.read().values() {
        tcp.send(&data.serialize())?
    }

    // end the handshake
    debug!("[TCP] [7] Finishing");
    tcp.send(&Flush::serialize())?;

    Ok(addr)
}

fn _handle_alive(tcp: TcpClient) -> Result<()> {
    let mut buf = [0; 1];
    let spin = SpinSleeper::default();

    loop {
        let t = Instant::now();

        // ping client
        tcp.send(&Ping::serialize())?;

        // wait for response
        tcp.recv(&mut buf)?;

        // enforce minimum ping
        let elapsed = t.elapsed();
        if elapsed < PING_MINIMUM {
            let diff = PING_MINIMUM - elapsed;
            spin.sleep(diff);
        }
    }
}

fn handle_alive(
    tcp: TcpClient,
    [addr_tcp, addr_udp]: [SocketAddr; 2],
    clients_tcp: TcpClients,
    clients_udp: UdpClients,
    sender: Sender<Packet>,
) -> JoinHandle<Result> {
    spawn(move || {
        if let Err(Error::Blazed(BlazedError::Io(e))) = _handle_alive(tcp) {
            if let std::io::ErrorKind::ConnectionReset = e.kind() {
                info!("{addr_tcp} has left")
            } else {
                warn!("{e}")
            }
        }

        if let Some(user) = clients_udp.write().remove(&addr_udp) {
            let id = user.id;

            // remove client before send packet to TCP channel
            clients_tcp.write().remove(&id);

            // send packet to TCP channel
            sender.send(RemObj { id }.serialize().to_vec())?
        }
        Ok(())
    })
}

fn handle_dist(s: &SyncSelect, clients_tcp: TcpClients, receiver_packet: Receiver<Packet>) {
    s.spawn(move || -> Result {
        loop {
            let packet = receiver_packet.recv()?;

            // distribute updates
            for tcp in clients_tcp.read().values() {
                tcp.send(&packet)?;
            }
        }
    });
}

fn handle_incoming(
    s: &SyncSelect,
    tcp_listener: TcpServer,
    clients_tcp: TcpClients,
    clients_udp: UdpClients,
    sender_packet: Sender<Packet>,
    receiver_addr: Receiver<SocketAddr>,
    updates: Updates,
    id: Arc<AtomicId>,
) {
    s.spawn(move || -> Result {
        for tcp in tcp_listener.incoming() {
            // the client's tcp address
            let addr_tcp = tcp.peer_addr()?;
            info!("{addr_tcp} attempting to join");

            // init handshake process
            match handshake(
                tcp.clone(),
                clients_udp.clone(),
                &receiver_addr,
                id.load(Ordering::Relaxed),
            ) {
                Ok(addr_udp) => {
                    info!("{addr_tcp} has joined");

                    // increment if handshake was successful
                    let id = id.fetch_add(1, Ordering::Relaxed);

                    // contruct client's initial object data
                    let data = UptObj {
                        id,
                        kind: ObjType::Player,
                        dim: Vec3::new(1.0, 1.0, 1.0),
                        color: Color::new([1.0, 1.0, 1.0, 1.0], false),
                        ..Default::default()
                    };

                    // initialize object update structure
                    updates.lock().insert(
                        addr_udp,
                        UptObjOpt {
                            id,
                            ..Default::default()
                        },
                    );

                    // add client stream to TCP table
                    clients_tcp.write().insert(id, tcp.clone());

                    // add player data to UDP table
                    clients_udp.write().insert(addr_udp, data);

                    // player joined
                    sender_packet.send(data.serialize().to_vec())?;

                    let _alive = handle_alive(
                        tcp,
                        [addr_tcp, addr_udp],
                        clients_tcp.clone(),
                        clients_udp.clone(),
                        sender_packet.clone(),
                    );
                }
                Err(e) => error!("[handle_incoming] {e:?}"),
            }
        }
        unreachable!()
    });
}

pub fn init_tcp(
    s: &SyncSelect,
    tcp: TcpServer,
    clients_tcp: TcpClients,
    clients_udp: UdpClients,
    sender_packet: Sender<Packet>,
    receiver_addr: Receiver<SocketAddr>,
    receiver_packet: Receiver<Packet>,
    updates: Updates,
    id: Arc<AtomicId>,
) {
    s.spawn_with(move |s| -> Result {
        handle_incoming(
            s,
            tcp,
            clients_tcp.clone(),
            clients_udp,
            sender_packet,
            receiver_addr,
            updates,
            id,
        );

        // init TCP distribution thread
        handle_dist(s, clients_tcp, receiver_packet);

        Ok(())
    });
}
