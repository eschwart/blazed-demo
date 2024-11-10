use crate::*;

use crossbeam_channel::{Receiver, Sender};

fn handshake(
    tcp: &TcpClient,
    clients_udp: UdpClients,
    receiver_addr: &Receiver<SocketAddr>,
    id: u8,
) -> Result<SocketAddr> {
    debug!("TCP [ ][1] Receiving handshake");
    let mut buf = [0; PACKET_SIZE];
    tcp.recv::<PacketKind, Packet, PACKET_SIZE>(&mut buf, PacketKind::Handshake)?
        .into_client_handshake()?;

    // reply with server handshake
    debug!("TCP [ ][2] Sending handshake");
    tcp.send(&Packet::Handshake(Handshake::server(id)))?;

    debug!("TCP [3][5] Waiting for UDP address");
    let addr = receiver_addr.recv()?;

    debug!("TCP [ ][6] Sending gamestates");
    for &player in clients_udp.read().values() {
        tcp.send(&Packet::Player(player))?;
    }
    debug!("TCP [ ][7] Finishing");
    tcp.send(&Packet::Flush)?;

    Ok(addr)
}

fn _handle_alive(tcp: &TcpClient) -> Result<()> {
    let mut buf = [0; PACKET_SIZE];
    let spinner = SpinSleeper::default();

    loop {
        tcp.recv::<_, Packet, PACKET_SIZE>(&mut buf, PacketKind::Ping)?;
        tcp.send(&Packet::Ping)?;

        spinner.sleep(PING_DELAY);
    }
}

fn handle_alive(
    tcp: TcpClient,
    addr: SocketAddr,
    clients_tcp: TcpClients,
    clients_udp: UdpClients,
    sender: Sender<Packet>,
) -> JoinHandle<Result> {
    spawn(move || {
        if let Err(e) = _handle_alive(&tcp) {
            warn!("{:?}", e)
        }

        if let Some(user) = clients_udp.write().remove(&addr) {
            let id = user.id();

            // remove client before send packet to TCP channel
            clients_tcp.write().remove(&id);

            // send packet to TCP channel
            sender.send(Packet::Remove(id))?
        }
        Ok(())
    })
}

fn handle_dist(s: &SyncSelect, clients_tcp: TcpClients, receiver_packet: Receiver<Packet>) {
    s.spawn(move || -> Result {
        // distribute updates
        loop {
            let packet = receiver_packet.recv()?;

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
    sender_client: Sender<Packet>,
    receiver_addr: Receiver<SocketAddr>,
    id: Arc<AtomicU8>,
) {
    s.spawn(move || -> Result {
        for tcp in tcp_listener.incoming() {
            let tcp_clone = if let Ok(clone) = tcp.try_clone() {
                clone
            } else {
                debug!("Failed to clone {:?}", tcp.stream());
                continue;
            };

            match handshake(
                &tcp,
                clients_udp.clone(),
                &receiver_addr,
                id.load(Ordering::Relaxed),
            ) {
                Ok(addr) => {
                    debug!("TCP [ ][8] Handshake complete");

                    // increment if handshake was successful
                    let id = id.fetch_add(1, Ordering::Relaxed);

                    // contruct client's player
                    let player = Player::new(id);

                    // add client stream to TCP table
                    clients_tcp.write().insert(id, tcp_clone);

                    // add player data to UDP table
                    clients_udp.write().insert(addr, player);

                    // player joined
                    sender_client.send(Packet::Player(player))?;

                    _ = handle_alive(
                        tcp,
                        addr,
                        clients_tcp.clone(),
                        clients_udp.clone(),
                        sender_client.clone(),
                    );
                }
                Err(e) => error!("[handle_incoming] {:?}", e),
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
    id: Arc<AtomicU8>,
) {
    s.spawn_with(move |s| -> Result {
        handle_incoming(
            s,
            tcp,
            clients_tcp.clone(),
            clients_udp,
            sender_packet,
            receiver_addr,
            id,
        );

        // init TCP distribution thread
        handle_dist(s, clients_tcp, receiver_packet);

        Ok(())
    });
}
