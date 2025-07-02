use crate::*;
use std::sync::atomic::AtomicU16;

pub fn handle_udp(
    s: &SyncSelect,
    udp: UdpClient,
    render_sender: Sender<()>,
    event_sender: Sender<GameEvent>,
    tps: RawTps,
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
            let n = udp.recv(&mut buf)?;
            let bytes = &buf[1..n];

            if buf[0] == UptObjOpt::ID {
                let data = UptObjOpt::deserialize(bytes);

                let action = if id == data.id {
                    ObjectAction::User { data }
                } else {
                    ObjectAction::Upt { data }
                };
                event_sender.send(GameEvent::Object(action))?;
                _ = render_sender.try_send(());
            }

            // increment TPS
            rate.fetch_add(1, Ordering::Relaxed);
        }
    });
}
