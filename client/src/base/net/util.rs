use crate::*;

use std::{collections::hash_map::Entry, thread::sleep};

use crossbeam_channel::{Receiver, Sender};

pub fn handshake(
    tcp: &TcpClient,
    udp: &mut UdpClient,
    cam: Camera,
    players: Players,
    event_sender: &Sender<GameEvent>,
) -> Result<u8> {
    debug!("[TCP] [][1] Sending client handshake");
    tcp.send(&Packet::Handshake(Handshake::client()))?;

    // widely used packet buffer
    let mut buf = [0; PACKET_SIZE];

    debug!("[TCP] [][2] Receiving server handshake");
    let id = tcp
        .recv::<PacketKind, Packet, PACKET_SIZE>(&mut buf, PacketKind::Handshake)?
        .into_server_handshake()?
        .id();

    debug!("[UDP] [][3] Sending client handshake");
    udp.send(&Packet::Handshake(Handshake::client()))?;

    debug!("[TCP] [][4] Receiving gamestates");
    while let Ok(packet) = tcp.recv(&mut buf, PacketKind::Player | PacketKind::Flush) {
        match packet {
            Packet::Player(player) => {
                handle_player_update(cam.clone(), players.clone(), event_sender, player, id)?
            }
            Packet::Flush => break,
            _ => unreachable!(),
        }
    }
    debug!("[TCP] [][5] Finishing");
    Ok(id)
}

pub fn handle_conn(
    cam: Camera,
    players: Players,
    input_receiver: Receiver<Input>,
    render_sender: Sender<()>,
    event_sender: Sender<GameEvent>,
    (tps, ping): (Tps, Ping),
    cfg: &ServerConfig,
) -> Result<()> {
    // establish connection
    debug!("[TCP] Connecting");
    let tcp = TcpClient::new(cfg.remote_tcp_addr())?;

    debug!("[UDP] Connecting");
    let mut udp = UdpClient::new(cfg.local_udp_addr(), cfg.remote_udp_addr())?;
    let udp_clone = udp.try_clone()?;

    let id = handshake(&tcp, &mut udp, cam.clone(), players.clone(), &event_sender)?;
    debug!("Handshake complete");

    let s = SyncSelect::default();

    // handle outgoing TCP packets
    handle_tcp(
        &s,
        tcp,
        cam.clone(),
        players.clone(),
        render_sender.clone(),
        event_sender.clone(),
        ping,
        id,
    );

    // handle outgoing UDP packets
    handle_udp(
        &s,
        udp_clone,
        cam.clone(),
        players.clone(),
        render_sender.clone(),
        event_sender,
        tps,
        id,
    );

    // handle mouse and keyboard input
    handle_input(&s, udp, input_receiver);

    Ok(())
}

pub fn init_conn(
    s: &SyncSelect,
    cam: Camera,
    players: Players,
    input_receiver: Receiver<Input>,
    render_sender: Sender<()>,
    event_sender: Sender<GameEvent>,
    stats: (Tps, Ping),
    cfg: ServerConfig,
) {
    s.spawn(move || -> Result {
        loop {
            // initialize TCP and UDP connection
            let result = handle_conn(
                cam.clone(),
                players.clone(),
                input_receiver.clone(),
                render_sender.clone(),
                event_sender.clone(),
                stats.clone(),
                &cfg,
            );

            // handle result
            if let Err(e) = result {
                error!("[init_conn] {}", e);
            }

            // reset game state
            event_sender.send(GameEvent::Reset)?;

            // reconnect timeout
            sleep(SECOND);
        }
    });
}

pub fn handle_player_update(
    cam: Camera,
    players: Players,
    event_sender: &Sender<GameEvent>,
    player: Player,
    id: u8,
) -> Result {
    if player.id() == id {
        // update camera if up-to-date
        let mut cam = cam.write();
        cam.attr = player.attr();
        cam.upt();
    } else {
        // modify player entry
        let mut players = players.write();
        let entry = players
            .entry(player.id())
            .and_modify(|p| *p.attr_mut() = player.attr());

        // create object if vacant
        if let Entry::Vacant(vacancy) = entry {
            // only render other players
            event_sender.send(GameEvent::Object(ObjectAction::Add(player)))?;
            vacancy.insert(player);
        }
    }
    Ok(())
}

pub fn handle_input(s: &SyncSelect, mut udp: UdpClient, input_receiver: Receiver<Input>) {
    s.spawn(move || -> Result<()> {
        udp.socket().set_write_timeout(Some(MILISECOND))?;

        // repeatedly send user input to server
        while let Ok(input) = input_receiver.recv() {
            udp.send(&Packet::Input(input))?;
        }
        Err(BlazedError::Infallible.into())
    });
}
