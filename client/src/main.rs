#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod base;

use base::*;
use crossbeam_channel::{Receiver, Sender, bounded};
use glow::{FILL, FRONT_AND_BACK, HasContext, LINE};
use sdl2::{
    EventPump, TimerSubsystem,
    event::{Event, EventSender, WindowEvent},
    keyboard::Keycode,
    video::{SwapInterval, Window},
};
use std::{
    sync::{Arc, atomic::Ordering},
    thread::{JoinHandle, spawn},
    time::Duration,
};
use sync_select::*;
use ultraviolet::Vec3;

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

// TOP-LEVEL THREAD (the godfather)
fn process_raw_events(
    gl: &GL,
    programs: &Shaders,
    window: Window,
    timer_fps_cfg: &mut impl FnMut(u8) -> u8,
    mut ep: EventPump,
    (cam, objects, state): (Camera, ObjectsRef, RenderState),
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
                            objects.write().retain(gl, ObjType::Player);
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
                                    obj.transform_upt();

                                    objects.write().insert(obj);
                                }
                                ObjectAction::Remove { id } => {
                                    if let Some(obj) = objects.write().remove(id) {
                                        free_buffers(gl, obj.buffers());
                                    }
                                }
                                ObjectAction::Upt { mut data } => {
                                    if let Some(obj) = objects.write().get_mut(data.id) {
                                        if data.cam.is_modified() {
                                            obj.cam.patch(&mut data.cam);
                                        }
                                        if let Some(dim) = data.dim {
                                            obj.dim = dim
                                        }
                                        obj.transform_upt();
                                    } else {
                                        error!("Object {} doesn't exist.", data.id)
                                    }
                                }
                                ObjectAction::User { mut data } => {
                                    if data.cam.is_modified() {
                                        let mut cam = cam.write();
                                        cam.attr_mut().patch(&mut data.cam);
                                        cam.upt();
                                    }
                                }
                            };
                        }
                        GameEvent::User(action) => {
                            match action {
                                UserAction::Keyboard(kb) => cam.write().input(kb),
                                UserAction::Wheel(Wheel { precise_y }) => {
                                    cam.write().upt_fov(precise_y)
                                }
                                UserAction::Motion(Motion { xrel, yrel }) => {
                                    cam.write().look_at(xrel, yrel)
                                }
                            };
                        }
                        // set a new fps target
                        GameEvent::Fps(fps) => {
                            // old and new rendering states
                            let old_state = timer_fps_cfg(fps) == 0;
                            let new_state = fps == 0;

                            // only reload the rendering state if fps mode is changed
                            if old_state != new_state {
                                state.store(RenderStateKind::Reload, Ordering::Relaxed);
                            }
                        }
                    };
                }
            }

            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => {
                state.store(RenderStateKind::Quit, Ordering::Release);
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

                    if keys.contains(Keys::RIGHT) {
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
    env_logger::init(); // instantiating logger

    // program arguments
    let cfg = Config::default();

    // init sdl and config
    // gl - needs to stay main thread
    let (sdl, video, timer, gl, window, ev, ep, _ctx) = init()?;
    video.gl_set_swap_interval(SwapInterval::Immediate)?;
    sdl.mouse().set_relative_mouse_mode(true);
    ev.register_custom_event::<GameEvent>()?;

    // program shaders
    let programs = init_shaders(&gl)?;

    ///////////////////////////////////////////////////////////////////////////////////////////
    // custom frames/second handler
    let (spin, limit, fps): (SpinSleeper, Limit, RawFps) = Default::default();
    let freq = timer.performance_frequency() as f32;

    // reset the fps counter every second
    let fps_clone = fps.clone();
    let _reset = spawn(move || {
        loop {
            spin.sleep(SECOND);
            fps_clone.reset();
        }
    });

    // individual frame facilitation channels
    let (fps_sender_1, fps_receiver_1) = bounded::<()>(1);
    let (fps_sender_2, fps_receiver_2) = bounded::<()>(1);

    // frame verification process
    let fps_clone = fps.clone();
    let limit_clone = limit.clone();
    let fps_timer_cb = || {
        // signaled to start frame
        fps_receiver_1.recv()?;
        let start = timer.performance_counter();

        // signaled to stop frame
        fps_receiver_1.recv()?;

        {
            let end = timer.performance_counter();

            let elapsed_sec = (end - start) as f32 / freq;
            let elapsed_dur = Duration::from_secs_f32(elapsed_sec);

            let limit = limit_clone.get();
            if elapsed_dur < limit {
                let dif = limit - elapsed_dur;
                spin.sleep(dif)
            }
            fps_clone.incr();
        }
        // signal to continue
        fps_sender_2.send(())?;

        Ok::<_, SyncError>(1)
    };

    // construct callback as official timer
    let mut timer_fps = None;

    // constructs/deconstructs the timer callback
    let fps_clone = fps.clone();
    let mut timer_fps_cfg = |fps: u8| -> u8 {
        limit.set(fps);
        let prev = fps_clone.swap_target(fps);

        timer_fps = if fps > 0 {
            Some(timer.add_timer(
                limit.get().as_millis() as u32,
                Box::new(|| fps_timer_cb().unwrap_or_default()),
            ))
        } else {
            None
        };
        prev
    };

    // initial timer config
    timer_fps_cfg(cfg.fps());
    ///////////////////////////////////////////////////////////////////////////////////////////

    // real-time user input
    let (raw_event_sender, raw_event_receiver) = bounded::<RawEvent>(32);
    let (event_sender, event_receiver) = bounded::<GameEvent>(32);

    // object storage manager
    let objects = {
        let mut raw = RawObjects::default();

        // basic 'light' structure
        raw.new_light(
            &gl,
            programs.simple(),
            -128,
            ObjType::Basic,
            Vec3::new(3.0, 2.0, -4.0),                  // position
            Vec3::new(0.5, 0.5, 0.5),                   // dimensions
            Color::new([1.0, 1.0, 0.8, 1.0], true),     // color
        )?;

        // basic 'land' structure
        raw.new_cube(
            &gl,
            programs.normal(),
            -127,
            ObjType::Basic,
            Vec3::new(0.0, -2.0, 0.0),
            Vec3::new(7.5, 0.1, 7.5),
            Color::new([1.0, 0.5, 0.31, 0.9], false),
        )?;

        Objects::new(raw)
    };

    // the user's camera
    let cam = Camera::new(window.size());

    // determinant of the status of threads
    let state: RenderState = Arc::new(AtomicRenderStateKind::new(RenderStateKind::Pass));

    // short-circuiting local thread manager
    let s = SyncSelect::default();

    // handle SIGINT
    handle_ctrlc(&s, event_sender.clone())?;

    // facilitate game events
    handle_game_events(&s, event_receiver, ev.event_sender());

    // input & network handling
    render_loop(
        &s,
        state.clone(),
        (raw_event_receiver, event_sender.clone()),
        (fps_sender_1, fps_receiver_2),
        (fps, RawTps::default(), RawPing::default()),
        cfg,
    );

    // post short-circuitry handler
    let _ss = handle_sync_select(s, event_sender);

    // main thread
    if let Err(e) = process_raw_events(
        &gl,
        &programs,
        window,
        &mut timer_fps_cfg,
        ep,
        (cam, &objects, state),
        raw_event_sender,
    ) {
        error!("{}", e)
    }

    // clean everything up
    clean_up(&gl, programs, objects.read().iter());

    Ok(())
}
