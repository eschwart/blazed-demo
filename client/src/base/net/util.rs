use crate::*;
use crossbeam_channel::{Receiver, Sender};
use pfrs::*;
use std::{net::Ipv4Addr, sync::atomic::AtomicU16, thread::sleep};

/// obtain player identity and gamestates from server
pub fn handshake(
    tcp: &TcpClient,
    udp: &mut UdpClient,
    event_sender: Arc<EventSender>,
) -> Result<Id> {
    debug!("[TCP] [1] Sending client handshake");
    tcp.send(&ClientHandshake::serialize())?;

    let mut buf = [0; PACKET_SIZE];

    debug!("[TCP] [2] Receiving server handshake");
    let n = tcp.recv(&mut buf)?;
    assert!(n != 0);

    if buf[0] != ServerHandshake::ID {
        return Err(format!("[TCP] [4] Found {}, expected ServerHandshake", buf[0]).into());
    }
    let id = ServerHandshake::deserialize(&buf[1..n]).id();

    debug!("[UDP] [3] Sending client handshake");
    udp.send(&ClientHandshake::serialize())?;

    debug!("[TCP] [4] Receiving game states");
    while let Ok(n) = tcp.recv(&mut buf) {
        assert!(n != 0);

        match buf[0] {
            Flush::ID => break,
            UptObj::ID => {
                let data = UptObj::deserialize(&buf[1..n]);
                event_sender.push_custom_event(GameEvent::Object(ObjectAction::Add { data }))?;
            }
            _ => unreachable!(),
        }
    }
    debug!("[TCP] [5] Finishing");

    // initial rendering
    event_sender.push_custom_event(GameEvent::Render(RenderAction::Flush))?;

    Ok(id)
}

/// send channeled user-input to UDP socket
pub fn handle_input(s: &SyncSelect, mut udp: UdpClient, input_receiver: Receiver<Vec<u8>>) {
    s.spawn(move || -> Result<()> {
        udp.socket().set_write_timeout(Some(GAME_SPEED))?;

        // repeatedly send user input to server
        while let Ok(data) = input_receiver.recv() {
            udp.send(data.as_slice())?;
        }
        Err(BlazedError::Infallible.into())
    });
}

/// establish client-server handshake then initialize TCP/UDP threads and input thread
pub fn handle_conn(
    event_sender: Arc<EventSender>,
    render_sender: Sender<()>,
    input_receiver: Receiver<Vec<u8>>,
    (tps, ping): (Arc<AtomicU16>, Arc<RwLock<Duration>>),
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

    // packet buffer for this client
    let id = handshake(&tcp, &mut udp, event_sender.clone())?;

    let s = SyncSelect::default();

    // handle outgoing TCP packets
    handle_tcp(
        &s,
        event_sender.clone(),
        render_sender.clone(),
        tcp,
        ping,
        id,
    );

    // handle outgoing UDP packets
    handle_udp(
        &s,
        event_sender,
        render_sender.clone(),
        udp.clone(),
        tps,
        id,
    );

    // handle mouse and keyboard input
    handle_input(&s, udp, input_receiver);

    Ok(())
}

pub fn init_conn(
    s: &SyncSelect,
    event_sender: Arc<EventSender>,
    render_sender: Sender<()>,
    input_receiver: Receiver<Vec<u8>>,
    stats: (Arc<AtomicU16>, Arc<RwLock<Duration>>),
    cfg: Config,
) {
    s.spawn(move || -> Result {
        let event_sender = event_sender;

        loop {
            // initialize TCP and UDP connection
            let result = handle_conn(
                event_sender.clone(),
                render_sender.clone(),
                input_receiver.clone(),
                stats.clone(),
                &cfg,
            );

            // handle result
            if let Err(e) = result {
                error!("[init_conn] {e}");
            }

            // reset game state
            event_sender.push_custom_event(GameEvent::Reset)?;

            // reconnect timeout
            sleep(SECOND);
        }
    });
}
