use crate::*;
use crossbeam_channel::Receiver;
use glow::{ARRAY_BUFFER, COLOR_BUFFER_BIT, Context, DEPTH_BUFFER_BIT, HasContext, NativeProgram};
use std::{
    io::{Write, stdout},
    sync::atomic::AtomicU16,
};
use sync_select::*;

fn setup_simple_obj(gl: &Context, program: NativeProgram, view: &[f32], projection: &[f32]) {
    unsafe {
        // view matrix
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(program, "view").as_ref(),
            false,
            view,
        );

        // projection matrix
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(program, "proj").as_ref(),
            false,
            projection,
        )
    };
}

fn setup_normal_obj<'a>(
    gl: &Context,
    program: NativeProgram,
    cam_pos: &[f32],
    lights: impl Iterator<Item = &'a (usize, (Vec3, Vec3))>,
    lights_len: usize,
) {
    unsafe {
        // projection matrix
        gl.uniform_3_f32_slice(
            gl.get_uniform_location(program, "cam_pos").as_ref(),
            cam_pos,
        );

        gl.uniform_1_i32(
            gl.get_uniform_location(program, "p_lights_len").as_ref(),
            lights_len as i32,
        );
    };
    for (i, (pos, col)) in lights {
        // set specific indexed light_pos[i]
        let light_name = format!("p_lights[{i}]");
        let light_pos_name = format!("{light_name}.pos");
        let light_col_name = format!("{light_name}.col");

        unsafe {
            let loc = gl.get_uniform_location(program, &light_pos_name);
            gl.uniform_3_f32_slice(loc.as_ref(), pos.as_slice());

            let loc = gl.get_uniform_location(program, &light_col_name);
            gl.uniform_3_f32_slice(loc.as_ref(), col.as_slice());
        }
    }
}

/// RENDER EVERYTHING
///
/// TODO - impl multiple lights
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
            let program = inst_group.program();
            let native = program.native();

            gl.use_program(Some(native));

            // view and proj
            setup_simple_obj(gl, native, view, projection);

            // D_Lights | P_Lights
            if let ProgramUnit::Normal = program.kind() {
                setup_normal_obj(gl, native, cam_pos_slice, lights.iter(), lights.len());
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

            // clean up (is this required?)
            gl.bind_vertex_array(None);
            gl.bind_buffer(ARRAY_BUFFER, None);
            gl.use_program(None);
        }
        // swap window
        window.gl_swap_window();

        // for precise frame time (benchmarking)
        #[cfg(debug_assertions)]
        gl.finish();
    }
}

fn handle_raw_events(
    s: &SyncSelect,
    (fps, ft, tps, ping): (
        Arc<Fps>,
        Arc<RwLock<Duration>>,
        Arc<AtomicU16>,
        Arc<RwLock<Duration>>,
    ),
    (mw_sender, mm_sender, kb_sender): (Sender<Wheel>, Sender<MotionOpt>, Sender<(Keys, bool)>),
    (raw_event_receiver, event_sender): (Receiver<RawEvent>, Sender<GameEvent>),
) {
    s.spawn(move || -> Result {
        let mut out = stdout();

        for event in raw_event_receiver.into_iter() {
            match event {
                RawEvent::Quit => break,
                RawEvent::MouseWheel(precise_y) => {
                    _ = mw_sender.try_send(Wheel { precise_y });
                }
                RawEvent::MouseMotion(xrel, yrel) => {
                    let opt = MotionOpt {
                        xrel: (xrel != 0).then_some(xrel),
                        yrel: (yrel != 0).then_some(yrel),
                    };
                    _ = mm_sender.try_send(opt);
                }
                RawEvent::Keyboard(kb, is_pressed) => {
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
                RawEvent::AspectRatio(w, h) => {
                    event_sender.try_send(GameEvent::Render(RenderAction::AspectRatio { w, h }))?
                }
            }
        }
        Ok(())
    });
}

/// Facilitate all user input.
fn process_input(
    s: &SyncSelect,
    (mw_receiver, mm_receiver, kb_receiver): (
        Receiver<Wheel>,
        Receiver<MotionOpt>,
        Receiver<(Keys, bool)>,
    ),
    render_sender: Sender<()>,
    input_sender: Sender<Vec<u8>>,
    event_sender: Sender<GameEvent>,
) {
    fn advance(render_sender: &Sender<()>, spinner: SpinSleeper) {
        // notify renderer
        _ = render_sender.try_send(());

        // this is the client's game speed
        spinner.sleep(GAME_SPEED);
    }

    fn process_mw(
        s: &SyncSelect,
        mw_receiver: Receiver<Wheel>,
        render_sender: Sender<()>,
        event_sender: Sender<GameEvent>,
        input_sender: Sender<Vec<u8>>,
    ) -> JoinHandle<Result> {
        s.spawn(move || -> Result {
            let spinner = SpinSleeper::default();

            loop {
                let wheel = mw_receiver.recv()?;
                event_sender.send(GameEvent::User(UserAction::Wheel(wheel)))?;

                advance(&render_sender, spinner);
                _ = input_sender.try_send(wheel.serialize().to_vec());
            }
        })
    }

    fn process_mm(
        s: &SyncSelect,
        mm_receiver: Receiver<MotionOpt>,
        render_sender: Sender<()>,
        event_sender: Sender<GameEvent>,
        input_sender: Sender<Vec<u8>>,
    ) -> JoinHandle<Result> {
        s.spawn(move || -> Result {
            let spinner = SpinSleeper::default();

            loop {
                let opt = mm_receiver.recv()?;
                let motion = Motion {
                    xrel: opt.xrel.unwrap_or_default(),
                    yrel: opt.yrel.unwrap_or_default(),
                };
                event_sender.send(GameEvent::User(UserAction::Motion(motion)))?;

                advance(&render_sender, spinner);
                _ = input_sender.try_send(opt.serialize().to_vec());
            }
        })
    }

    fn process_kb(
        s: &SyncSelect,
        kb_receiver: Receiver<(Keys, bool)>,
        render_sender: Sender<()>,
        event_sender: Sender<GameEvent>,
        input_sender: Sender<Vec<u8>>,
    ) -> JoinHandle<Result> {
        fn cont_handler(
            s: &SyncSelect,
            waiter: Waiter,
            keys_cont: AtomicKeys,
            event_sender: Sender<GameEvent>,
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
                        event_sender.send(GameEvent::User(UserAction::Keyboard(kb)))?;

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
            event_sender: Sender<GameEvent>,
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
                        event_sender.send(GameEvent::User(UserAction::Keyboard(keys_norm)))?;
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
        mw_receiver,
        render_sender.clone(),
        event_sender.clone(),
        input_sender.clone(),
    );

    // process mouse motion input
    process_mm(
        s,
        mm_receiver,
        render_sender.clone(),
        event_sender.clone(),
        input_sender.clone(),
    );

    // process keyboard input
    process_kb(s, kb_receiver, render_sender, event_sender, input_sender);
}

fn handle_rendering_uncapped(
    s: &SyncSelect,
    state: RenderState,
    fps: Arc<Fps>,
    event_sender: Sender<GameEvent>,
    render_receiver: Receiver<()>,
) -> JoinHandle<RenderStateKind> {
    let render = move || -> Result {
        event_sender.send(GameEvent::Render(RenderAction::Flush))?;

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

fn handle_rendering_capped(
    s: &SyncSelect,
    state: RenderState,
    event_sender: Sender<GameEvent>,
    render_receiver: Receiver<()>,
    (fps_sender, fps_receiver): (Sender<()>, Receiver<()>),
) -> JoinHandle<RenderStateKind> {
    let render = move || -> Result {
        // signal to start
        fps_sender.send(())?;

        // render objects
        event_sender.send(GameEvent::Render(RenderAction::Flush))?;

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
    state: RenderState,
    (raw_event_receiver, event_sender): (Receiver<RawEvent>, Sender<GameEvent>),
    channel_fps: (Sender<()>, Receiver<()>),
    (fps, ft, tps, ping): (
        Arc<Fps>,
        Arc<RwLock<Duration>>,
        Arc<AtomicU16>,
        Arc<RwLock<Duration>>,
    ),
    cfg: Config,
) {
    let (mm_sender, mm_receiver) = bounded(1);
    let (mw_sender, mw_receiver) = bounded(1);
    let (kb_sender, kb_receiver) = bounded(1);

    let (render_sender, render_receiver) = bounded(1);
    let (input_sender, input_receiver) = bounded(1);

    if cfg.is_online() {
        // NETWORKING
        //
        // init TCP and UDP threads
        init_conn(
            s,
            input_receiver,
            render_sender.clone(),
            event_sender.clone(),
            (tps.clone(), ping.clone()),
            cfg,
        );
    }

    // handle input throughput
    handle_raw_events(
        s,
        (fps.clone(), ft, tps, ping),
        (mm_sender, mw_sender, kb_sender),
        (raw_event_receiver, event_sender.clone()),
    );

    // process current input in real-time
    process_input(
        s,
        (mm_receiver, mw_receiver, kb_receiver),
        render_sender,
        input_sender,
        event_sender.clone(),
    );

    // facilitate frame renders
    s.spawn(move || -> Result {
        let s = SyncSelect::default();

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
    });
}
