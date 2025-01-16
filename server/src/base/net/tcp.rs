use crate::*;
use crossbeam_channel::{Receiver, Sender};

fn handshake(
    tcp: &TcpClient,
    clients_udp: UdpClients,
    receiver_addr: &Receiver<SocketAddr>,
    id: Id,
) -> Result<SocketAddr> {
    debug!("TCP [ ][1] Receiving handshake");
    let mut buf = [0; PACKET_SIZE];
    tcp.recv::<PacketKind, Packet, PACKET_SIZE>(&mut buf, PacketKind::Handshake)?
        .into_client_handshake()?;

    // reply with server handshake
    debug!("TCP [ ][2] Sending handshake");
    tcp.send(&Packet::Handshake {
        handshake: Handshake::server(id),
    })?;

    debug!("TCP [3][5] Waiting for UDP address");
    let addr = receiver_addr.recv()?;

    debug!("TCP [ ][6] Sending gamestates");
    for &data in clients_udp.read().values() {
        tcp.send(&Packet::AddObj { data })?;
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

        spinner.sleep(PING_MINIMUM);
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
            sender.send(Packet::RemObj { id })?
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
    id: Arc<AtomicId>,
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

                    // contruct client's initial object data
                    let data = ObjectData::new(
                        id,
                        Color::new([0.1, 0.6, 1.0, 1.0], false),
                        RawObjectData::Player(PlayerData::new(Vector::zeros())),
                    );

                    // add client stream to TCP table
                    clients_tcp.write().insert(id, tcp_clone);

                    // add player data to UDP table
                    clients_udp.write().insert(addr, data);

                    // player joined
                    sender_packet.send(Packet::AddObj { data })?;

                    _ = handle_alive(
                        tcp,
                        addr,
                        clients_tcp.clone(),
                        clients_udp.clone(),
                        sender_packet.clone(),
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
            id,
        );

        // init TCP distribution thread
        handle_dist(s, clients_tcp, receiver_packet);

        Ok(())
    });
}
