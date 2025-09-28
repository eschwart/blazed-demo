use crate::*;
use crossbeam_channel::Sender;
use std::time::Instant;

fn ping_handler(s: &SyncSelect, ping: Arc<RwLock<Duration>>, rate: Arc<RwLock<Duration>>) {
    s.spawn(move || {
        let spin = SpinSleeper::default();

        loop {
            spin.sleep(SECOND);
            let value = *rate.read();
            *rate.write() = Duration::ZERO;
            *ping.write() = value;
        }
    });
}

pub fn handle_tcp(
    s: &SyncSelect,
    event_sender: Arc<EventSender>,
    render_sender: Sender<()>,
    tcp: TcpClient,
    ping: Arc<RwLock<Duration>>,
    id: Id,
) {
    let rate: Arc<RwLock<Duration>> = Default::default();

    ping_handler(s, ping, rate.clone());

    s.spawn(move || -> Result {
        let mut buf = [0; PACKET_SIZE];

        loop {
            // init timer
            let t = Instant::now();

            // wait for response
            let n = tcp.recv(&mut buf)?;
            let bytes = &buf[1..n];

            match buf[0] {
                Ping::ID => tcp.send(&Ping::serialize())?,

                RemObj::ID => {
                    let data = RemObj::deserialize(bytes);
                    event_sender.push_custom_event(GameEvent::Object(ObjectAction::Remove {
                        id: data.id,
                    }))?;
                    _ = render_sender.try_send(());
                }
                UptObj::ID => {
                    let data = UptObj::deserialize(bytes);

                    let action = if id == data.id {
                        let cam_opt = CameraAttrOpt {
                            fov: Some(data.cam.fov),
                            speed: Some(data.cam.speed),
                            yaw: Some(data.cam.yaw),
                            pitch: Some(data.cam.pitch),
                            eye: Some(data.cam.eye),
                            target: Some(data.cam.target),
                            up: Some(data.cam.up),
                        };

                        let data = UptObjOpt {
                            id,
                            kind: Some(data.kind),
                            dim: Some(data.dim),
                            color: Some(data.color),
                            cam: cam_opt,
                            keys: data.keys,
                        };
                        ObjectAction::User { data }
                    } else {
                        ObjectAction::Add { data }
                    };
                    event_sender.push_custom_event(GameEvent::Object(action))?;
                    _ = render_sender.try_send(());
                }

                _ => (),
            }

            // update ping nonetheless
            *rate.write() = t.elapsed();
        }
    });
}
