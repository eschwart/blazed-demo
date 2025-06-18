use crate::*;
use crossbeam_channel::Receiver;
use glow::{COLOR_BUFFER_BIT, Context, DEPTH_BUFFER_BIT, HasContext, NativeProgram};
use std::io::{Write, stdout};
use sync_select::*;

fn setup_simple_obj(
    gl: &Context,
    native: NativeProgram,
    model: &[f32],
    view: &[f32],
    projection: &[f32],
    obj_col: &[f32],
) {
    // model matrix
    unsafe {
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(native, "model").as_ref(),
            false,
            model,
        )
    };

    // view matrix
    unsafe {
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(native, "view").as_ref(),
            false,
            view,
        )
    };

    // projection matrix
    unsafe {
        gl.uniform_matrix_4_f32_slice(
            gl.get_uniform_location(native, "proj").as_ref(),
            false,
            projection,
        )
    };

    // object color
    unsafe { gl.uniform_4_f32_slice(gl.get_uniform_location(native, "obj_col").as_ref(), obj_col) };
}

fn setup_normal_obj(
    gl: &Context,
    native: NativeProgram,
    cam_pos: &[f32],
    light_pos: &[f32],
    light_col: &[f32],
) {
    // camera position
    unsafe { gl.uniform_3_f32_slice(gl.get_uniform_location(native, "cam_pos").as_ref(), cam_pos) };

    // light position attibute
    unsafe {
        gl.uniform_3_f32_slice(
            gl.get_uniform_location(native, "light_pos").as_ref(),
            light_pos,
        )
    };

    // light color attribute
    unsafe {
        gl.uniform_3_f32_slice(
            gl.get_uniform_location(native, "light_col").as_ref(),
            light_col,
        )
    };
}

/// TODO - impl instanced rendering
fn render_obj(
    gl: &Context,
    obj: &Object,
    model: &[f32],
    view: &[f32],
    projection: &[f32],
    cam_pos: &[f32],
    light_pos: &[f32],
    light_col: &[f32],
) {
    // current program
    let program = obj.program();
    let native = program.native();
    unsafe { gl.use_program(Some(native)) };

    // every object inherits the attributes of a 'simple' shader
    setup_simple_obj(gl, native, model, view, projection, &obj.color());

    // 'normal' (ambient + diffuse + specular) shading
    if program.kind() == ProgramUnit::Normal {
        setup_normal_obj(gl, native, cam_pos, light_pos, light_col);
    }

    // bind then render
    unsafe { gl.bind_vertex_array(Some(obj.vao())) };
    unsafe { gl.draw_elements(obj.mode(), obj.len(), obj.element_type(), 0) };

    // clean up
    unsafe { gl.bind_vertex_array(None) };
    unsafe { gl.use_program(None) };
}

/// TODO - impl multiple lights
pub fn display(gl: &Context, window: &Window, cam: &RawCamera, objects: &RawObjects) {
    unsafe {
        gl.clear_color(0.4, 0.4, 0.4, 1.0);
        gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

        // camera attributes
        let view = cam.view().as_slice();
        let projection = cam.projection().as_slice();
        let cam_pos = cam.pos().as_slice();

        // light attributes
        let light = objects.lights().next().unwrap(); /////////////////////////////////////////// TODO
        let pos = light.cam.eye;
        let light_pos = pos.as_slice();
        let light_col = &light.color()[..3];

        // render lights then the rest
        objects.lights().chain(objects.opaque()).for_each(|obj| {
            let model = obj.tran().model();

            render_obj(
                gl,
                obj,
                model.as_slice(),
                view,
                projection,
                cam_pos,
                light_pos,
                light_col,
            );
        });
        // swap window
        window.gl_swap_window();
    }
}

fn handle_raw_events(
    s: &SyncSelect,
    (fps, tps, ping): (RawFps, RawTps, RawPing),
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
    fps: RawFps,
    event_sender: Sender<GameEvent>,
    render_receiver: Receiver<()>,
) -> JoinHandle<RenderStateKind> {
    let render = move || -> Result {
        event_sender.send(GameEvent::Render(RenderAction::Flush))?;

        // incrememnt frame count
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
    (fps, tps, ping): (RawFps, RawTps, RawPing),
    cfg: Config,
) {
    let (mm_sender, mm_receiver) = bounded(1);
    let (mw_sender, mw_receiver) = bounded(1);
    let (kb_sender, kb_receiver) = bounded(1);

    let (render_sender, render_receiver) = bounded(1);
    let (input_sender, input_receiver) = bounded(1);

    if cfg.is_online() {
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
        (fps.clone(), tps, ping),
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
