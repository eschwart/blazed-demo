use crate::*;
use crossbeam_channel::{Receiver, Sender};
use glow::{ARRAY_BUFFER, COLOR_BUFFER_BIT, Context, DEPTH_BUFFER_BIT, HasContext};
use std::{
    io::{Write, stdout},
    sync::atomic::AtomicU16,
};
use sync_select::*;

fn setup_simple_obj(gl: &Context, prog: Program, view: &[f32], proj: &[f32]) {
    unsafe {
        // view matrix
        gl.program_uniform_matrix_4_f32_slice(prog.native(), prog.unif_locs().view(), false, view);

        // projection matrix
        gl.program_uniform_matrix_4_f32_slice(prog.native(), prog.unif_locs().proj(), false, proj)
    };
}

fn setup_normal_obj<'a>(
    gl: &Context,
    prog: Program,
    cam_pos: &[f32],
    lights: impl Iterator<Item = &'a (usize, (Vec3, Vec3))>,
    lights_len: usize,
) {
    unsafe {
        // projection matrix
        gl.program_uniform_3_f32_slice(prog.native(), prog.unif_locs().cam_pos(), cam_pos);

        // number of point lights
        gl.program_uniform_1_i32(
            prog.native(),
            prog.unif_locs().p_lights_len(),
            lights_len as i32,
        );
    };
    let p_lights_idxs = prog.unif_locs().p_lights();
    for (i, (pos, col)) in lights {
        let [pos_idx, col_idx] = p_lights_idxs[*i];
        unsafe {
            gl.program_uniform_3_f32_slice(prog.native(), pos_idx.as_ref(), pos.as_slice());
            gl.program_uniform_3_f32_slice(prog.native(), col_idx.as_ref(), col.as_slice());
        }
    }
}

/// RENDER EVERYTHING
pub fn display(gl: &Context, window: &Window, cam: &RawCamera, objects: &RawObjects) {
    unsafe {
        gl.clear_color(0.4, 0.4, 0.4, 1.0);
        gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

        // camera attributes
        let view = cam.view().as_slice();
        let projection = cam.projection().as_slice();
        let cam_pos = cam.attr().eye;
        let cam_pos_slice = cam_pos.as_slice();

        // TODO - improve this.. (try not to collect everytime?)
        let lights = objects
            .lights()
            .enumerate()
            .collect::<Vec<(usize, (Vec3, Vec3))>>();

        // render each groups objects
        for inst_group in objects.groups() {
            let prog = inst_group.program();
            gl.use_program(Some(prog.native())); // still required

            // view and proj
            setup_simple_obj(gl, prog, view, projection);

            // D_Lights | P_Lights
            if let ProgramKind::Normal = prog.kind() {
                setup_normal_obj(gl, prog, cam_pos_slice, lights.iter(), lights.len());
            }

            // TODO - improve this (only update transformations that have changed)
            {
                // gather all model matrices and color vectors for this group
                let mut model_matrices: Vec<f32> =
                    Vec::with_capacity(inst_group.instances().len() * 16);
                let mut color_data: Vec<f32> = Vec::with_capacity(inst_group.instances().len() * 4);
                for instance in inst_group.instances().iter() {
                    model_matrices.extend_from_slice(instance.trans.model().as_slice());
                    color_data.extend_from_slice(instance.color.data().as_slice());
                }
                // update color VBO
                gl.bind_buffer(ARRAY_BUFFER, Some(inst_group.data().col_vbo()));
                gl.buffer_data_u8_slice(
                    ARRAY_BUFFER,
                    bytemuck::cast_slice(&color_data),
                    glow::DYNAMIC_DRAW,
                );

                // update instance VBO
                gl.bind_buffer(ARRAY_BUFFER, Some(inst_group.data().inst_vbo()));
                gl.buffer_data_u8_slice(
                    ARRAY_BUFFER,
                    bytemuck::cast_slice(&model_matrices),
                    glow::DYNAMIC_DRAW,
                );
            }
            // bind then render
            gl.bind_vertex_array(Some(inst_group.data().vao()));
            gl.draw_elements_instanced(
                inst_group.data().mode(),
                inst_group.data().len(),
                inst_group.data().element_type(),
                0,
                inst_group.len() as i32,
            );

            // not necessary with the current implementation
            #[cfg(debug_assertions)]
            {
                gl.use_program(None);
                gl.bind_vertex_array(None);
                gl.bind_buffer(ARRAY_BUFFER, None);
            }
        }
        // swap window
        window.gl_swap_window();

        // for precise frame time (benchmarking)
        #[cfg(debug_assertions)]
        gl.finish();
    }
}

fn handle_sys_events(
    s: &SyncSelect,
    (fps, ft, tps, ping): (
        Arc<Fps>,
        Arc<RwLock<Duration>>,
        Arc<AtomicU16>,
        Arc<RwLock<Duration>>,
    ),
    (mw_sender, mm_sender, kb_sender): (Sender<Wheel>, Sender<MotionOpt>, Sender<(Keys, bool)>),
    (event_sender, sys_event_receiver): (Arc<EventSender>, Receiver<SysEvent>),
) {
    s.spawn(move || -> Result {
        let mut out = stdout();

        for event in sys_event_receiver.into_iter() {
            match event {
                SysEvent::Quit => break,
                SysEvent::MouseWheel(precise_y) => {
                    _ = mw_sender.try_send(Wheel { precise_y });
                }
                SysEvent::MouseMotion(xrel, yrel) => {
                    let opt = MotionOpt {
                        xrel: (xrel != 0).then_some(xrel),
                        yrel: (yrel != 0).then_some(yrel),
                    };
                    _ = mm_sender.try_send(opt);
                }
                SysEvent::Keyboard(kb, is_pressed) => {
                    if kb.contains(Keys::LEFT) {
                        let fps = fps.get();
                        let ft = *ft.read();
                        let tps = tps.load(Ordering::Relaxed);
                        let ping = *ping.read();

                        let msg = format!("\r{{ Fps: {fps} @ {ft:?}, Tps: {tps}, Ping {ping:?} }}");

                        if let Err(e) = out.write_all(msg.as_bytes()) {
                            error!("{e}")
                        } else {
                            _ = out.flush()
                        }
                        continue;
                    }
                    _ = kb_sender.try_send((kb, is_pressed));
                }
                SysEvent::AspectRatio(w, h) => event_sender
                    .push_custom_event(GameEvent::Render(RenderAction::AspectRatio { w, h }))?,
            }
        }
        Ok(())
    });
}

/// Facilitate all user input.
fn process_input(
    s: &SyncSelect,
    event_sender: Arc<EventSender>,
    input_sender: Sender<Vec<u8>>,
    render_sender: Sender<()>,
    (mw_receiver, mm_receiver, kb_receiver): (
        Receiver<Wheel>,
        Receiver<MotionOpt>,
        Receiver<(Keys, bool)>,
    ),
) {
    fn advance(render_sender: &Sender<()>, spinner: SpinSleeper) {
        // notify renderer
        _ = render_sender.try_send(());

        // this is the client's game speed
        spinner.sleep(GAME_SPEED);
    }

    fn process_mw(
        s: &SyncSelect,
        event_sender: Arc<EventSender>,
        input_sender: Sender<Vec<u8>>,
        render_sender: Sender<()>,
        mw_receiver: Receiver<Wheel>,
    ) -> JoinHandle<Result> {
        s.spawn(move || -> Result {
            let spinner = SpinSleeper::default();

            loop {
                let wheel = mw_receiver.recv()?;
                event_sender.push_custom_event(GameEvent::User(UserAction::Wheel(wheel)))?;

                advance(&render_sender, spinner);
                _ = input_sender.try_send(wheel.serialize().to_vec());
            }
        })
    }

    fn process_mm(
        s: &SyncSelect,
        event_sender: Arc<EventSender>,
        input_sender: Sender<Vec<u8>>,
        render_sender: Sender<()>,
        mm_receiver: Receiver<MotionOpt>,
    ) -> JoinHandle<Result> {
        s.spawn(move || -> Result {
            let spinner = SpinSleeper::default();

            loop {
                let opt = mm_receiver.recv()?;
                let motion = Motion {
                    xrel: opt.xrel.unwrap_or_default(),
                    yrel: opt.yrel.unwrap_or_default(),
                };
                event_sender.push_custom_event(GameEvent::User(UserAction::Motion(motion)))?;

                advance(&render_sender, spinner);
                _ = input_sender.try_send(opt.serialize().to_vec());
            }
        })
    }

    fn process_kb(
        s: &SyncSelect,
        event_sender: Arc<EventSender>,
        input_sender: Sender<Vec<u8>>,
        render_sender: Sender<()>,
        kb_receiver: Receiver<(Keys, bool)>,
    ) -> JoinHandle<Result> {
        fn cont_handler(
            s: &SyncSelect,
            waiter: Waiter,
            keys_cont: AtomicKeys,
            event_sender: Arc<EventSender>,
            render_sender: Sender<()>,
        ) -> JoinHandle<Result> {
            s.spawn(move || {
                let spinner: SpinSleeper = Default::default();
                loop {
                    // wait for continuous input
                    waiter.wait();

                    loop {
                        // obtain current key
                        let kb = keys_cont.get();
                        if kb.is_empty() {
                            break;
                        }
                        // send input to client event handler
                        event_sender
                            .push_custom_event(GameEvent::User(UserAction::Keyboard(kb)))?;

                        // render the change
                        advance(&render_sender, spinner);
                    }

                    waiter.reset();
                }
            })
        }

        fn kb_handler(
            s: &SyncSelect,
            notifier: Notifier,
            mut keys_cont: AtomicKeys,
            kb_receiver: Receiver<(Keys, bool)>,
            event_sender: Arc<EventSender>,
            input_sender: Sender<Vec<u8>>,
        ) -> JoinHandle<Result> {
            s.spawn(move || {
                let mut keys_norm = Keys::default();

                loop {
                    // wait for keyboard change
                    let (key, is_pressed) = kb_receiver.recv()?;

                    // send change to server
                    _ = input_sender.try_send(
                        Keyboard {
                            bits: key.bits(),
                            is_pressed: is_pressed as u8,
                        }
                        .serialize()
                        .to_vec(),
                    );

                    // facilitate continuous/non-continuous keystrokes
                    if key.is_continuous(KeyState::Player) {
                        if is_pressed {
                            // if continuous input was empty before the currently active keypress
                            let cont_was_empty = keys_cont.is_empty();

                            // add the current keypress
                            keys_cont |= key;

                            // only unpark if the continuous input is no longer empty
                            if cont_was_empty {
                                notifier.notify();
                            }
                        } else {
                            // remove the key no longer being pressed
                            keys_cont -= key;
                        }
                    // similar logic here
                    } else {
                        if is_pressed {
                            keys_norm |= key;
                        } else {
                            keys_norm -= key
                        }
                        event_sender
                            .push_custom_event(GameEvent::User(UserAction::Keyboard(keys_norm)))?;
                    }
                }
            })
        }
        let waiter = Waiter::default();
        let notifier = waiter.notifier();

        let keys_cont = AtomicKeys::default();
        cont_handler(
            s,
            waiter,
            keys_cont.clone(),
            event_sender.clone(),
            render_sender.clone(),
        );
        kb_handler(
            s,
            notifier,
            keys_cont,
            kb_receiver,
            event_sender,
            input_sender,
        )
    }

    // process mouse wheel input
    process_mw(
        s,
        event_sender.clone(),
        input_sender.clone(),
        render_sender.clone(),
        mw_receiver,
    );

    // process mouse motion input
    process_mm(
        s,
        event_sender.clone(),
        input_sender.clone(),
        render_sender.clone(),
        mm_receiver,
    );

    // process keyboard input
    process_kb(s, event_sender, input_sender, render_sender, kb_receiver);
}

pub fn handle_rendering_uncapped(
    s: &SyncSelect,
    state: RenderState,
    fps: Arc<Fps>,
    event_sender: Arc<EventSender>,
    render_receiver: Receiver<()>,
) -> JoinHandle<RenderStateKind> {
    let render = move || -> Result {
        event_sender.push_custom_event(GameEvent::Render(RenderAction::Flush))?;

        // increment frame count
        fps.incr();

        Ok(())
    };

    s.spawn(move || {
        loop {
            let state = state.load(Ordering::Relaxed);

            match state {
                RenderStateKind::Pass => {
                    _ = render(); // ignore errors

                    // exit if required threads are dead
                    if render_receiver.recv().is_err() {
                        break RenderStateKind::Quit;
                    }
                }
                _ => break state,
            }
        }
    })
}

pub fn handle_rendering_capped(
    s: &SyncSelect,
    state: RenderState,
    event_sender: Arc<EventSender>,
    render_receiver: Receiver<()>,
    (fps_sender, fps_receiver): (Sender<()>, Receiver<()>),
) -> JoinHandle<RenderStateKind> {
    let render = move || -> Result {
        // signal to start
        fps_sender.send(())?;

        // render objects
        event_sender.push_custom_event(GameEvent::Render(RenderAction::Flush))?;

        // signal to stop
        fps_sender.send(())?;

        // respect frame rate
        fps_receiver.recv()?;

        Ok(())
    };

    s.spawn(move || {
        loop {
            let state = state.load(Ordering::Relaxed);

            match state {
                RenderStateKind::Pass => {
                    _ = render(); // ignore errors

                    // exit if required threads are dead
                    if render_receiver.recv().is_err() {
                        break RenderStateKind::Quit;
                    }
                }
                _ => break state,
            }
        }
    })
}

pub fn render_loop(
    s: &SyncSelect,
    (event_sender, fps_sender): (Arc<EventSender>, Sender<()>),
    (sys_event_receiver, fps_receiver): (Receiver<SysEvent>, Receiver<()>),
    (fps, ft, tps, ping, state): (
        Arc<Fps>,
        Arc<RwLock<Duration>>,
        Arc<AtomicU16>,
        Arc<RwLock<Duration>>,
        Arc<AtomicRenderStateKind>,
    ),
    cfg: Config,
) -> JoinHandle<Result> {
    let (mm_sender, mm_receiver) = bounded(1);
    let (mw_sender, mw_receiver) = bounded(1);
    let (kb_sender, kb_receiver) = bounded(1);

    let (input_sender, input_receiver) = bounded(1);
    let (render_sender, render_receiver) = bounded(1);

    // Networking
    if cfg.is_online() {
        // init TCP and UDP threads
        init_conn(
            s,
            event_sender.clone(),
            render_sender.clone(),
            input_receiver,
            (tps.clone(), ping.clone()),
            cfg,
        );
    }

    // handle input throughput
    handle_sys_events(
        s,
        (fps.clone(), ft, tps, ping),
        (mm_sender, mw_sender, kb_sender),
        (event_sender.clone(), sys_event_receiver),
    );

    // process current input in real-time
    process_input(
        s,
        event_sender.clone(),
        input_sender,
        render_sender,
        (mm_receiver, mw_receiver, kb_receiver),
    );

    // facilitate frame renders
    s.spawn(move || -> Result {
        let s = SyncSelect::default();
        let channel_fps = (fps_sender, fps_receiver);

        loop {
            let state_kind = if fps.target() > 0 {
                handle_rendering_capped(
                    &s,
                    state.clone(),
                    event_sender.clone(),
                    render_receiver.clone(),
                    channel_fps.clone(),
                )
            } else {
                handle_rendering_uncapped(
                    &s,
                    state.clone(),
                    fps.clone(),
                    event_sender.clone(),
                    render_receiver.clone(),
                )
            }
            .join()?;

            match state_kind {
                RenderStateKind::Reload => {
                    state.store(RenderStateKind::Pass, Ordering::Relaxed);
                }
                RenderStateKind::Quit => break Ok(()),
                _ => unreachable!(),
            }
        }
    })
}
