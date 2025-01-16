use crate::*;
use std::sync::atomic::AtomicU16;

pub fn handle_udp(
    s: &SyncSelect,
    udp: UdpClient,
    render_sender: Sender<()>,
    event_sender: Sender<GameEvent>,
    tps: Tps,
    id: Id,
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
            let packet: Packet = udp.recv(&mut buf, PacketKind::UptObj)?;

            if let Packet::UptObj { data } = packet {
                handle_obj(id, ObjectAction::Upt { data }, &event_sender)?;
                _ = render_sender.try_send(());
            }
            rate.fetch_add(1, Ordering::Relaxed);
        }
    });
}
