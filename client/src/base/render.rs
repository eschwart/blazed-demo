use crate::*;

use std::io::{stdout, Write};

use crossbeam_channel::Receiver;
use glow::{Context, HasContext, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT, TRIANGLES, UNSIGNED_SHORT};

unsafe fn render_obj(
    gl: &Context,
    cam: &RawCamera,
    obj: &Object,
    rotation: Matrix,
    translation: Matrix,
) {
    // current program
    let p = obj.program();
    gl.use_program(Some(p));

    // model matrix
    gl.uniform_matrix_4_f32_slice(
        Some(&gl.get_uniform_location(p, "model").unwrap()),
        false,
        cam.model(),
    );

    // view matrix
    gl.uniform_matrix_4_f32_slice(
        Some(&gl.get_uniform_location(p, "view").unwrap()),
        false,
        cam.view(),
    );

    // projection matrix
    gl.uniform_matrix_4_f32_slice(
        Some(&gl.get_uniform_location(p, "projection").unwrap()),
        false,
        cam.projection(),
    );

    // translation
    gl.uniform_matrix_4_f32_slice(
        Some(&gl.get_uniform_location(p, "translation").unwrap()),
        false,
        translation.as_slice(),
    );

    // rotation
    gl.uniform_matrix_4_f32_slice(
        Some(&gl.get_uniform_location(p, "rotation").unwrap()),
        false,
        rotation.as_slice(),
    );

    // camera position (arbitrary position for now)
    gl.uniform_3_f32_slice(
        Some(&gl.get_uniform_location(p, "light_pos").unwrap()),
        &[5.0; 3],
    );

    // object color
    gl.uniform_4_f32_slice(
        Some(&gl.get_uniform_location(p, "color").unwrap()),
        obj.color(),
    );

    gl.uniform_1_f32(
        Some(&gl.get_uniform_location(p, "near").unwrap()),
        cam.near(),
    );
    gl.uniform_1_f32(Some(&gl.get_uniform_location(p, "far").unwrap()), cam.far());

    // bind then render
    gl.bind_vertex_array(Some(obj.vao()));
    gl.draw_elements(TRIANGLES, obj.len(), UNSIGNED_SHORT, 0);

    // clean up
    gl.bind_vertex_array(None);
    gl.use_program(None);
}

pub fn display<'a>(
    gl: &Context,
    cam: RwLockReadGuard<RawCamera>,
    objects: impl Iterator<Item = &'a Object>,
    players: RwLockReadGuard<'_, HashMap<u8, Player>>,
) {
    unsafe {
        gl.clear_color(0.2, 0.2, 0.2, 1.0);
        gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

        for obj in objects {
            // determine transformations
            let (rotation, translation) = {
                players.get(&obj.id()).map_or(
                    (
                        Rotation::default().to_homogeneous(),
                        Translation::default().to_homogeneous(),
                    ),
                    |p| {
                        let attr = p.attr();
                        (attr.rotation(), attr.translation())
                    },
                )
            };
            // render the provided object
            render_obj(gl, &cam, obj, rotation, translation);
        }
    }
}

fn handle_input(
    s: &SyncSelect,
    keys: Keys,
    cam: Camera,
    (fps, tps, ping): (Fps, Tps, Ping),
    (ms_sender, kb_sender, render_sender, input_sender, raw_event_receiver): (
        Sender<()>,
        Sender<()>,
        Sender<()>,
        Sender<Input>,
        Receiver<RawEvent>,
    ),
) {
    s.spawn(move || -> Result {
        let mut out = stdout();

        for event in raw_event_receiver.into_iter() {
            match event {
                RawEvent::Quit => break,
                RawEvent::MouseWheel(precise_y) => {
                    cam.write().upt_fov(precise_y);
                    _ = ms_sender.try_send(());
                }
                RawEvent::MouseMotion(xrel, yrel) => {
                    cam.write().look_at(xrel, yrel);

                    // notify input handler
                    _ = ms_sender.try_send(());

                    _ = input_sender.try_send(Input::Mouse { xrel, yrel });
                }
                RawEvent::Keyboard(flags, pressed) => {
                    if flags.contains(Flags::LEFT) {
                        let fps = fps.get();
                        let tps = tps.load(Ordering::Relaxed);
                        let ping = *ping.read();

                        let msg = format!("\r{{ Fps: {}, Tps: {}, Ping {:?} }}", fps, tps, ping);

                        if let Err(e) = out.write_all(msg.as_bytes()) {
                            error!("{}", e)
                        } else {
                            _ = out.flush()
                        }
                        continue;
                    }

                    if pressed {
                        *keys.write() |= flags;

                        // notify input handler
                        _ = kb_sender.try_send(());
                    } else {
                        *keys.write() -= flags;
                    }
                }
                RawEvent::AspectRatio(w, h) => {
                    cam.write().upt_aspect_ratio(w, h);
                    _ = render_sender.send(());
                }
            }
        }
        // notify rendering thread in order to break it
        _ = render_sender.send(());

        Ok(())
    });
}

fn process_input(
    s: &SyncSelect,
    (keys, cam): (Keys, Camera),
    (ms_receiver, kb_receiver, render_sender, input_sender): (
        Receiver<()>,
        Receiver<()>,
        Sender<()>,
        Sender<Input>,
    ),
) {
    fn advance(render_sender: &Sender<()>, spinner: SpinSleeper) {
        // notify renderer
        _ = render_sender.try_send(());

        // this is technically the game speed (client-side)
        spinner.sleep(MILISECOND);
    }

    fn process_ms(s: &SyncSelect, (ms_receiver, render_sender): (Receiver<()>, Sender<()>)) {
        s.spawn(move || -> Result {
            let spinner = SpinSleeper::default();

            loop {
                ms_receiver.recv()?;
                advance(&render_sender, spinner);
            }
        });
    }

    fn process_kb(
        s: &SyncSelect,
        (keys, cam): (Keys, Camera),
        (kb_receiver, render_sender, input_sender): (Receiver<()>, Sender<()>, Sender<Input>),
    ) {
        s.spawn(move || -> Result {
            let spinner = SpinSleeper::default();

            loop {
                kb_receiver.recv()?;

                loop {
                    let flags = *keys.read();

                    if flags.is_empty() {
                        break;
                    }

                    // update camera
                    cam.write().input(flags);

                    // render next tick
                    advance(&render_sender, spinner);

                    _ = input_sender.try_send(Input::Keyboard { keys: flags });
                }
            }
        });
    }

    // process mouse input
    process_ms(s, (ms_receiver, render_sender.clone()));

    // process keyboard input
    process_kb(s, (keys, cam), (kb_receiver, render_sender, input_sender));
}

fn handle_rendering(
    s: &SyncSelect,
    running: Running,
    event_sender: Sender<GameEvent>,
    (fps_sender, fps_receiver): (Sender<()>, Receiver<()>),
    render_receiver: Receiver<()>,
) {
    let handler = move || -> Result {
        // signal to start
        fps_sender.send(())?;

        // render objects
        event_sender.send(GameEvent::Render)?;

        // signal to stop
        fps_sender.send(())?;

        // respect frame rate
        fps_receiver.recv()?;

        Ok(())
    };

    s.spawn(move || -> Result {
        while running.load(Ordering::Relaxed) {
            _ = handler(); // ignore any fails

            // exit if required threads are dead
            if render_receiver.recv().is_err() {
                break;
            }
        }
        Ok(())
    });
}

pub fn render_loop(
    s: &SyncSelect,
    (cam, players, running): (Camera, Players, Running),
    (raw_event_receiver, event_sender): (Receiver<RawEvent>, Sender<GameEvent>),
    (fps_sender, fps_receiver): (Sender<()>, Receiver<()>),
    (fps, tps, ping): (Fps, Tps, Ping),
    cfg: Config,
) {
    let keys = Keys::default();

    let (ms_sender, ms_receiver) = bounded(1);
    let (kb_sender, kb_receiver) = bounded(1);

    let (render_sender, render_receiver) = bounded(1);
    let (input_sender, input_receiver) = bounded(1);

    // init TCP and UDP threads
    if let Some(cfg) = cfg.server() {
        init_conn(
            s,
            cam.clone(),
            players.clone(),
            input_receiver,
            render_sender.clone(),
            event_sender.clone(),
            (tps.clone(), ping.clone()),
            cfg,
        );
    }

    // handle input throughput
    handle_input(
        s,
        keys.clone(),
        cam.clone(),
        (fps, tps, ping),
        (
            ms_sender,
            kb_sender,
            render_sender.clone(),
            input_sender.clone(),
            raw_event_receiver,
        ),
    );

    // process current input in real-time
    process_input(
        s,
        (keys, cam),
        (ms_receiver, kb_receiver, render_sender, input_sender),
    );

    // render when told to
    handle_rendering(
        s,
        running,
        event_sender,
        (fps_sender, fps_receiver),
        render_receiver,
    );
}
