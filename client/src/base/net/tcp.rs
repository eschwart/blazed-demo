use super::*;
use crate::*;

use std::time::Instant;

pub fn handle_tcp(
    s: &SyncSelect,
    tcp: TcpClient,
    cam: Camera,
    players: Players,
    render_sender: Sender<()>,
    event_sender: Sender<GameEvent>,
    ping: Ping,
    id: u8,
) {
    let rate: Arc<RwLock<Duration>> = Default::default();
    let rate_clone = rate.clone();

    s.spawn(move || -> Result {
        let spinner = SpinSleeper::default();

        loop {
            spinner.sleep(SECOND);
            let rate = *rate_clone.read();
            *rate_clone.write() = Duration::default();
            *ping.write() = rate;
        }
    });

    s.spawn(move || -> Result {
        let mut buf = [0; PACKET_SIZE];

        loop {
            // init timer
            let time = Instant::now();

            // ping to server
            tcp.send(&Packet::Ping)?;

            // wait for response
            match tcp.recv(
                &mut buf,
                PacketKind::Player | PacketKind::Remove | PacketKind::Ping,
            )? {
                Packet::Player(player) => {
                    handle_player_update(cam.clone(), players.clone(), &event_sender, player, id)?;
                    _ = render_sender.try_send(());
                }

                Packet::Remove(id) => {
                    event_sender.send(GameEvent::Object(ObjectAction::Remove(id)))?;
                    _ = render_sender.try_send(());
                }
                Packet::Ping => (),
                _ => unreachable!(),
            }
            // update ping nonetheless
            *rate.write() = time.elapsed()
        }
    });
}
