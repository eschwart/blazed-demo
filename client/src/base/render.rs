use crate::*;
use crossbeam_channel::Receiver;
use glow::{Context, HasContext, NativeProgram, COLOR_BUFFER_BIT, DEPTH_BUFFER_BIT};
use std::io::{stdout, Write};
use sync_select::*;

unsafe fn setup_simple_obj(
    gl: &Context,
    native: NativeProgram,
    model: &[f32],
    view: &[f32],
    projection: &[f32],
    obj_col: &[f32],
) {
    // model matrix
    gl.uniform_matrix_4_f32_slice(
        gl.get_uniform_location(native, "model").as_ref(),
        false,
        model,
    );

    // view matrix
    gl.uniform_matrix_4_f32_slice(
        gl.get_uniform_location(native, "view").as_ref(),
        false,
        view,
    );

    // projection matrix
    gl.uniform_matrix_4_f32_slice(
        gl.get_uniform_location(native, "proj").as_ref(),
        false,
        projection,
    );

    // object color
    gl.uniform_4_f32_slice(gl.get_uniform_location(native, "obj_col").as_ref(), obj_col);
}

unsafe fn setup_normal_obj(
    gl: &Context,
    native: NativeProgram,
    view_pos: &[f32],
    light_pos: &[f32],
    light_col: &[f32],
) {
    // camera position
    gl.uniform_3_f32_slice(
        gl.get_uniform_location(native, "view_pos").as_ref(),
        view_pos,
    );

    // light position attibute
    gl.uniform_3_f32_slice(
        gl.get_uniform_location(native, "light_pos").as_ref(),
        light_pos,
    );

    // light color attribute
    gl.uniform_3_f32_slice(
        gl.get_uniform_location(native, "light_col").as_ref(),
        light_col,
    );
}

unsafe fn render_obj(
    gl: &Context,
    obj: &Object,
    model: &[f32],
    view: &[f32],
    projection: &[f32],
    view_pos: &[f32],
    light_pos: &[f32],
    light_col: &[f32],
) {
    // current program
    let program = obj.program();
    let native = program.native();
    gl.use_program(Some(native));

    // every object inherits the attributes of a 'simple' shader
    setup_simple_obj(gl, native, model, view, projection, obj.color());

    // 'normal' (ambient + diffuse + specular) shading
    if program.kind() == ProgramUnit::Normal {
        setup_normal_obj(gl, native, view_pos, light_pos, light_col);
    }

    // bind then render
    gl.bind_vertex_array(Some(obj.vao()));
    gl.draw_elements(obj.mode(), obj.len(), obj.element_type(), 0);

    // clean up
    gl.bind_vertex_array(None);
    gl.use_program(None);
}

pub fn display(gl: &Context, window: &Window, cam: &RawCamera, objects: &RawObjects) {
    unsafe {
        gl.clear_color(0.1, 0.1, 0.1, 1.0);
        gl.clear(COLOR_BUFFER_BIT | DEPTH_BUFFER_BIT);

        // camera attributes
        let view = cam.view().as_slice();
        let projection = cam.projection().as_slice();
        let view_pos = cam.pos().as_slice();

        // light attributes
        let light = objects.lights().next().unwrap(); /////////////////////////////////////////// TODO
        let light_pos = light.pos().as_slice();
        let light_col = &light.color()[..3];

        // render objects here (light obj last)
        objects.iter().for_each(|obj| {
            let model = obj.model().as_slice();

            render_obj(
                gl, obj, model, view, projection, view_pos, light_pos, light_col,
            );
        });
        // swap window
        window.gl_swap_window();
    }
}

fn handle_raw_events(
    s: &SyncSelect,
    keys: Keys,
    (fps, tps, ping): (Fps, Tps, Ping),
    (ms_sender, kb_sender): (Sender<Mouse>, Sender<()>),
    (raw_event_receiver, event_sender): (Receiver<RawEvent>, Sender<GameEvent>),
) {
    s.spawn(move || -> Result {
        let mut out = stdout();

        for event in raw_event_receiver.into_iter() {
            match event {
                RawEvent::Quit => break,
                RawEvent::MouseWheel(precise_y) => {
                    _ = ms_sender.try_send(Mouse::Wheel { precise_y });
                }
                RawEvent::MouseMotion(xrel, yrel) => {
                    _ = ms_sender.try_send(Mouse::Motion { xrel, yrel });
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
                    event_sender.try_send(GameEvent::Render(RenderAction::AspectRatio { w, h }))?
                }
            }
        }
        // this? something (rendering) used to be here but why [seems to work without it]
        Ok(())
    });
}

/// Facilitate all user input.
///
/// Non-continuous inputs (e.g., MouseWheel) are getting mitigated
/// by high throughput inputs (e.g., MouseMotion).
///
/// Bad Solution - increase size of input buffer(s) => increases latency.
///
/// Better Solution - implement a 3rd processor for miscilaneous (non-continuous) inputs.
/// Should allow for a more concurrent facilitation of inputs.
/// Still want to adhere to tick-rate (don't want to allow potential input abuse)
fn process_input(
    s: &SyncSelect,
    keys: Keys,
    ms_receiver: Receiver<Mouse>,
    kb_receiver: Receiver<()>,
    (ms_verify_receiver, kb_verify_receiver): (Receiver<bool>, Receiver<bool>),
    render_sender: Sender<()>,
    input_sender: Sender<Input>,
    event_sender: Sender<GameEvent>,
) {
    fn advance(render_sender: &Sender<()>, spinner: SpinSleeper) {
        // notify renderer
        _ = render_sender.try_send(());

        // this is the client's game speed
        spinner.sleep(TICK_RATE);
    }

    fn process_ms(
        s: &SyncSelect,
        ms_receiver: Receiver<Mouse>,
        ms_verify_receiver: Receiver<bool>,
        render_sender: Sender<()>,
        event_sender: Sender<GameEvent>,
        input_sender: Sender<Input>,
    ) {
        s.spawn(move || -> Result {
            let spinner = SpinSleeper::default();

            loop {
                let input = Input::Mouse(ms_receiver.recv()?);

                event_sender.send(GameEvent::User(UserAction::Input(input)))?;

                if ms_verify_receiver.recv()? {
                    advance(&render_sender, spinner);
                    _ = input_sender.try_send(input);
                }
            }
        });
    }

    fn process_kb(
        s: &SyncSelect,
        keys: Keys,
        kb_receiver: Receiver<()>,
        kb_verify_receiver: Receiver<bool>,
        render_sender: Sender<()>,
        event_sender: Sender<GameEvent>,
        input_sender: Sender<Input>,
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

                    // TODO (maybe) - filter out any non-continuous keys
                    // break if empty afterwards

                    // send movement event to event handler
                    event_sender
                        .send(GameEvent::User(UserAction::Input(Input::Keyboard(flags))))?;

                    // advance if movement was valid
                    if kb_verify_receiver.recv()? {
                        // render next tick
                        advance(&render_sender, spinner);

                        _ = input_sender.try_send(Input::Keyboard(flags));
                    }
                }
            }
        });
    }

    // process mouse input
    process_ms(
        s,
        ms_receiver,
        ms_verify_receiver,
        render_sender.clone(),
        event_sender.clone(),
        input_sender.clone(),
    );

    // process keyboard input
    process_kb(
        s,
        keys,
        kb_receiver,
        kb_verify_receiver,
        render_sender,
        event_sender,
        input_sender,
    );
}

fn handle_rendering(
    s: &SyncSelect,
    running: Arc<AtomicBool>,
    event_sender: Sender<GameEvent>,
    (fps_sender, fps_receiver): (Sender<()>, Receiver<()>),
    render_receiver: Receiver<()>,
) {
    let handler = move || -> Result {
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

    s.spawn(move || -> Result {
        while running.load(Ordering::Relaxed) {
            _ = handler(); // ignore errors

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
    running: Arc<AtomicBool>,
    (ms_verify_receiver, kb_verify_receiver): (Receiver<bool>, Receiver<bool>),
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
        keys.clone(),
        (fps, tps, ping),
        (ms_sender, kb_sender),
        (raw_event_receiver, event_sender.clone()),
    );

    // process current input in real-time
    process_input(
        s,
        keys,
        ms_receiver,
        kb_receiver,
        (ms_verify_receiver, kb_verify_receiver),
        render_sender,
        input_sender,
        event_sender.clone(),
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
