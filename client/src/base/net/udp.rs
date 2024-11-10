use super::*;
use crate::*;

use std::sync::atomic::AtomicU16;

pub fn handle_udp(
    s: &SyncSelect,
    udp: UdpClient,
    cam: Camera,
    players: Players,
    render_sender: Sender<()>,
    event_sender: Sender<GameEvent>,
    tps: Tps,
    id: u8,
) {
    let rate: Arc<AtomicU16> = Default::default();
    let rate_clone = rate.clone();

    s.spawn(move || -> Result {
        let spinner = SpinSleeper::default();

        loop {
            spinner.sleep(SECOND);
            let rate = rate_clone.swap(0, Ordering::Relaxed);
            tps.store(rate, Ordering::Relaxed);
        }
    });

    s.spawn(move || -> Result {
        let mut buf = [0; PACKET_SIZE];

        loop {
            // wait for server to send player update
            let packet: Packet = udp.recv(&mut buf, PacketKind::Player)?;

            if let Packet::Player(player) = packet {
                // handle update
                handle_player_update(cam.clone(), players.clone(), &event_sender, player, id)?;
                _ = render_sender.try_send(());
            }
            rate.fetch_add(1, Ordering::Relaxed);
        }
    });
}
