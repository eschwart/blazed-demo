use crate::*;
use crossbeam_channel::{Receiver, Sender};
use pfrs::*;

use std::{net::Ipv4Addr, thread::sleep};

pub fn handshake(
    tcp: &TcpClient,
    udp: &mut UdpClient,
    event_sender: &Sender<GameEvent>,
) -> Result<Id> {
    debug!("[TCP] [][1] Sending client handshake");
    tcp.send(&Packet::Handshake {
        handshake: Handshake::client(),
    })?;

    // widely used packet buffer
    let mut buf = [0; PACKET_SIZE];

    debug!("[TCP] [][2] Receiving server handshake");
    let id = tcp
        .recv::<PacketKind, Packet, PACKET_SIZE>(&mut buf, PacketKind::Handshake)?
        .into_server_handshake()?
        .id();

    debug!("[UDP] [][3] Sending client handshake");
    udp.send(&Packet::Handshake {
        handshake: Handshake::client(),
    })?;

    debug!("[TCP] [][4] Receiving gamestates");
    while let Ok(packet) = tcp.recv(&mut buf, PacketKind::AddObj | PacketKind::Flush) {
        match packet {
            Packet::AddObj { data } => handle_obj(id, ObjectAction::Add { data }, &event_sender)?,
            Packet::Flush => break,
            _ => unreachable!(),
        }
    }
    debug!("[TCP] [][5] Finishing");

    // initial rendering
    event_sender.send(GameEvent::Render(RenderAction::Flush))?;

    Ok(id)
}

fn map_obj_event(user_id: Id, action: ObjectAction) -> Result<GameEvent> {
    let game_event = match action {
        ObjectAction::Add { data } | ObjectAction::Upt { data } => (data.id() == user_id)
            .then_some({
                let data = data.player().ok_or("Expected 'Player' object type")?.data();
                GameEvent::Object(ObjectAction::User { data })
            }),

        ObjectAction::Rem { id } => (id == user_id).then_some(GameEvent::Reset),
        _ => unreachable!(),
    }
    .unwrap_or(GameEvent::Object(action));
    Ok(game_event)
}

pub fn handle_obj(user_id: Id, action: ObjectAction, event_sender: &Sender<GameEvent>) -> Result {
    let game_event = map_obj_event(user_id, action)?;
    event_sender.send(game_event).map_err(Into::into)
}

pub fn handle_conn(
    input_receiver: Receiver<Input>,
    render_sender: Sender<()>,
    event_sender: Sender<GameEvent>,
    (tps, ping): (Tps, Ping),
    cfg: &Config,
) -> Result<()> {
    // establish connection
    debug!("[TCP] Connecting");
    let tcp = TcpClient::new(cfg.remote_tcp_addr())?;

    debug!("[UDP] Connecting");
    let local_udp_addr = cfg.local_udp_addr().unwrap_or(get_socket_addr(
        find_open_port(Ipv4Addr::LOCALHOST, Protocol::Udp).ok_or("No available dynamic ports")?,
    ));
    let mut udp = UdpClient::new(local_udp_addr, cfg.remote_udp_addr())?;
    let udp_clone = udp.try_clone()?;

    let id = handshake(&tcp, &mut udp, &event_sender)?;
    debug!("Handshake complete");

    let s = SyncSelect::default();

    // handle outgoing TCP packets
    handle_tcp(
        &s,
        tcp,
        render_sender.clone(),
        event_sender.clone(),
        ping,
        id,
    );

    // handle outgoing UDP packets
    handle_udp(&s, udp_clone, render_sender.clone(), event_sender, tps, id);

    // handle mouse and keyboard input
    handle_input(&s, udp, input_receiver);

    Ok(())
}

pub fn init_conn(
    s: &SyncSelect,
    input_receiver: Receiver<Input>,
    render_sender: Sender<()>,
    event_sender: Sender<GameEvent>,
    stats: (Tps, Ping),
    cfg: Config,
) {
    s.spawn(move || -> Result {
        loop {
            // initialize TCP and UDP connection
            let result = handle_conn(
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

pub fn handle_input(s: &SyncSelect, mut udp: UdpClient, input_receiver: Receiver<Input>) {
    s.spawn(move || -> Result<()> {
        udp.socket().set_write_timeout(Some(TICK_RATE))?;

        // repeatedly send user input to server
        while let Ok(input) = input_receiver.recv() {
            udp.send(&Packet::Input { input })?;
        }
        Err(BlazedError::Infallible.into())
    });
}
