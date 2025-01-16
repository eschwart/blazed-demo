#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod base;

use base::*;
use crossbeam_channel::{bounded, Receiver, Sender};
use glow::{HasContext, FILL, FRONT_AND_BACK, LINE};
use sdl2::{
    event::{Event, EventSender, WindowEvent},
    keyboard::Keycode,
    video::{SwapInterval, Window},
    EventPump,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{spawn, JoinHandle},
    time::Duration,
};
use sync_select::*;

fn handle_sync_select(s: SyncSelect, event_sender: Sender<GameEvent>) -> JoinHandle<Result> {
    // unhandled new thread
    spawn(move || {
        s.join(); // wait for any thread to finish
        error!("[SyncSelect] A thread has unexpectedly finished.");
        event_sender.send(GameEvent::Quit).map_err(Into::into)
    })
}

fn handle_ctrlc(s: &SyncSelect, event_sender: Sender<GameEvent>) -> Result {
    let thread = s.thread();

    ctrlc::set_handler(move || {
        thread.unpark();

        if event_sender.send(GameEvent::Quit).is_err() {
            error!("Failed to notify event handler to quit")
        }
    })
    .map_err(Into::into)
}

fn handle_game_events(s: &SyncSelect, receiver: Receiver<GameEvent>, sender: EventSender) {
    s.spawn(move || -> Result {
        while let Ok(event) = receiver.recv() {
            _ = sender.push_custom_event(event);
        }
        Ok(())
    });
}

fn process_raw_events(
    gl: &GL,
    programs: &Shaders,
    window: Window,
    mut ep: EventPump,
    (cam, objects, running): (Camera, ObjectsRef, Arc<AtomicBool>),
    (ms_verify_sender, kb_verify_sender): (Sender<bool>, Sender<bool>),
    raw_event_sender: Sender<RawEvent>,
) -> Result {
    let mut mode = false;

    for event in ep.wait_iter() {
        match event {
            Event::User { .. } => {
                if let Some(user_event) = event.as_user_event_type::<GameEvent>() {
                    match user_event {
                        GameEvent::Quit => break,
                        GameEvent::Reset => {
                            // remove and deallocate all player objects
                            objects.write().retain(gl, RawObjectDataUnit::Player);
                        }
                        GameEvent::Render(action) => {
                            // usually window-based events
                            if let RenderAction::AspectRatio { w, h } = action {
                                unsafe {
                                    gl.viewport(0, 0, w, h);
                                }
                                cam.write().upt_aspect_ratio(w, h);
                            }
                            // render a frame
                            display(gl, &window, &cam.read(), &objects.read());
                        }
                        GameEvent::Object(action) => {
                            match action {
                                ObjectAction::Add { data } => {
                                    let mut obj =
                                        Object::create_cube_with(gl, programs.normal(), data)?;

                                    // initial transformations if player
                                    if obj.data().player_ref().is_some() {
                                        obj.data_mut().transform_upt();
                                    }

                                    objects.write().insert(obj);
                                }

                                ObjectAction::Rem { id } => {
                                    if let Some(obj) = objects.write().remove(id) {
                                        free_buffers(gl, obj.buffers());
                                    }
                                }

                                ObjectAction::Upt { data } => {
                                    if let Some(obj_data) = objects.write().get_mut(data.id()) {
                                        *obj_data = data;

                                        // update transformations if player
                                        if obj_data.player_ref().is_some() {
                                            obj_data.transform_upt();
                                        }
                                    }
                                }

                                ObjectAction::User { data } => cam.write().replace(data.attr()),
                            };
                        }
                        GameEvent::User(action) => {
                            match action {
                                UserAction::Input(input) => {
                                    match input {
                                        Input::Mouse(mouse) => match mouse {
                                            Mouse::Wheel { precise_y } => {
                                                cam.write().upt_fov(precise_y);
                                                ms_verify_sender.send(true)?;
                                            }
                                            Mouse::Motion { xrel, yrel } => {
                                                cam.write().look_at(xrel, yrel);
                                                ms_verify_sender.send(true)?;
                                            }
                                        },
                                        Input::Keyboard(flags) => {
                                            cam.write().input(flags);
                                            kb_verify_sender.send(true)?;
                                        }
                                    };
                                }
                            };
                        }
                    };
                }
            }

            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => {
                running.store(false, Ordering::Release);
                raw_event_sender.send(RawEvent::Quit)?;
                break;
            }
            Event::Window {
                win_event: WindowEvent::SizeChanged(w, h),
                ..
            } => raw_event_sender.send(RawEvent::AspectRatio(w, h))?,
            Event::MouseWheel { precise_y, .. } => {
                raw_event_sender.send(RawEvent::MouseWheel(precise_y))?
            }
            Event::MouseMotion { xrel, yrel, .. } => {
                raw_event_sender.send(RawEvent::MouseMotion(xrel, yrel))?
            }
            Event::KeyDown {
                scancode: Some(key),
                repeat: false,
                ..
            } => {
                if let Some(keys) = try_from_scancode(key) {
                    _ = raw_event_sender.try_send(RawEvent::Keyboard(keys, true));

                    if keys.contains(Flags::RIGHT) {
                        unsafe {
                            if mode {
                                gl.polygon_mode(FRONT_AND_BACK, FILL);
                            } else {
                                gl.polygon_mode(FRONT_AND_BACK, LINE);
                            }
                            mode = !mode;
                        }
                    }
                }
            }

            Event::KeyUp {
                scancode: Some(key),
                repeat: false,
                ..
            } => {
                if let Some(keys) = try_from_scancode(key) {
                    _ = raw_event_sender.try_send(RawEvent::Keyboard(keys, false))
                }
            }
            _ => (),
        }
    }
    Ok(())
}

fn main() -> Result {
    init_logger();
    let cfg = Config::default();

    // init sdl and config
    let (sdl, video, gl, window, ev, ep, _ctx) = init()?;
    video.gl_set_swap_interval(SwapInterval::Immediate)?;
    sdl.mouse().set_relative_mouse_mode(true);
    ev.register_custom_event::<GameEvent>()?;

    // program shaders
    let programs = init_shaders(&gl)?;

    // SDL's built-in timer subsystem
    let timer = sdl.timer()?;

    // custom frames/second handler
    let mut fps_counter = FPSCounter::new(&timer);
    fps_counter.set(cfg.fps());

    // individual frame facilitation channels
    let (fps_sender_1, fps_receiver_1) = bounded::<()>(1);
    let (fps_sender_2, fps_receiver_2) = bounded::<()>(1);

    // frame verification process
    let _fps_timer_cb = || {
        // signaled to start frame
        fps_receiver_1.recv()?;
        let start = fps_counter.start(&timer);

        // signaled to stop frame
        fps_receiver_1.recv()?;
        fps_counter.stop(&timer, start);

        // signal to continue
        fps_sender_2.send(())?;
        Ok::<_, SyncError>(1)
    };

    // construct verification callback as official timer
    let _fps_timer = timer.add_timer(
        fps_counter.delay(),
        Box::new(|| _fps_timer_cb().unwrap_or_default()),
    );

    // real-time user input
    let (raw_event_sender, raw_event_receiver) = bounded::<RawEvent>(32);
    let (event_sender, event_receiver) = bounded::<GameEvent>(32);

    // object storage manager
    let objects = {
        let mut raw = RawObjects::default();

        // basic 'light' structure
        raw.new_light(
            &gl,
            -128,
            programs.simple(),
            Vector::new(3.0, 2.0, -4.0),
            Vector::new(0.5, 0.5, 0.5),
            Color::new([1.0, 1.0, 0.8, 1.0], true),
        )?;

        // basic 'land' structure
        raw.new_cube(
            &gl,
            -127,
            programs.normal(),
            Vector::new(0.0, -2.0, 0.0),
            Vector::new(7.5, 0.1, 7.5),
            Color::new([1.0, 0.5, 0.31, 0.9], false),
            RawObjectDataUnit::Basic,
        )?;
        Objects::new(raw)
    };

    // the user's camera
    let cam = Camera::new(window.size());

    // mouse/keyboard facilitation channels
    let (ms_verify_sender, ms_verify_receiver) = bounded::<bool>(1);
    let (kb_verify_sender, kb_verify_receiver) = bounded::<bool>(1);

    // determinant of the status of threads
    let running: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));

    // short-circuiting local thread manager
    let s = SyncSelect::default();

    // handle SIGINT
    handle_ctrlc(&s, event_sender.clone())?;

    // facilitate game events
    handle_game_events(&s, event_receiver, ev.event_sender());

    // input & network handling
    render_loop(
        &s,
        running.clone(),
        (kb_verify_receiver, ms_verify_receiver),
        (raw_event_receiver, event_sender.clone()),
        (fps_sender_1, fps_receiver_2),
        (fps_counter.reader(), Tps::default(), Ping::default()),
        cfg,
    );

    // post short-circuitry handler
    let _ss = handle_sync_select(s, event_sender);

    // main thread
    if let Err(e) = process_raw_events(
        &gl,
        &programs,
        window,
        ep,
        (cam, &objects, running),
        (kb_verify_sender, ms_verify_sender),
        raw_event_sender,
    ) {
        error!("{}", e)
    }

    // clean everything up
    clean_up(&gl, programs, objects.read().iter());

    Ok(())
}
