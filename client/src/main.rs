#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod base;

use base::*;

use std::{
    collections::HashMap,
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::spawn,
    time::Duration,
};

use ::obj::{load_obj, Obj};
use crossbeam_channel::{bounded, Receiver, Sender};
use glow::{HasContext, Program};
use sdl2::{
    event::{Event, EventSender, WindowEvent},
    keyboard::Keycode,
    video::{FullscreenType, SwapInterval, Window},
    EventPump,
};

fn handle_game_events(s: &SyncSelect, receiver: Receiver<GameEvent>, sender: EventSender) {
    s.spawn(move || -> Result {
        while let Ok(event) = receiver.recv() {
            _ = sender.push_custom_event(event)
        }
        Ok(())
    });
}

fn handle_raw_events(
    gl: &GL,
    program: Program,
    mut window: Window,
    mut ep: EventPump,
    (cam, players, objects, running): (Camera, Players, &mut HashMap<u8, Object>, Running),
    raw_event_sender: Sender<RawEvent>,
    cube: Obj,
) -> Result {
    for event in ep.wait_iter() {
        match event {
            Event::User { .. } => {
                if let Some(user_event) = event.as_user_event_type::<GameEvent>() {
                    match user_event {
                        GameEvent::Quit => break,
                        GameEvent::Reset => {
                            // remove and deallocate all player objects
                            for id in players.write().drain().map(|p| p.0) {
                                if let Some(obj) = objects.remove(&id) {
                                    free_object(gl, &obj);
                                }
                            }
                        }
                        GameEvent::Render => {
                            display(gl, cam.read(), objects.values(), players.read());
                            window.gl_swap_window();
                        }
                        GameEvent::Object(action) => {
                            match action {
                                ObjectAction::Add(player) => {
                                    objects.insert(
                                        player.id(),
                                        Object::from_obj(
                                            gl,
                                            program,
                                            cube.clone(),
                                            [0.1, 0.6, 1.0, 1.0],
                                            player.id(),
                                        )?,
                                    );
                                }
                                ObjectAction::Remove(id) => {
                                    _ = players.write().remove(&id);
                                    if let Some(obj) = objects.remove(&id) {
                                        free_object(gl, &obj);
                                    }
                                }
                            };
                        }
                    }
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
            } => {
                unsafe {
                    gl.viewport(0, 0, w, h);
                }
                raw_event_sender.send(RawEvent::AspectRatio(w, h))?
            }
            Event::MouseWheel { precise_y, .. } => {
                raw_event_sender.send(RawEvent::MouseWheel(precise_y))?
            }
            Event::MouseMotion { xrel, yrel, .. } => {
                raw_event_sender.send(RawEvent::MouseMotion(-xrel, -yrel))?
            }
            Event::KeyDown {
                scancode: Some(key),
                repeat: false,
                ..
            } => {
                if let Some(keys) = try_from_scancode(key) {
                    _ = raw_event_sender.try_send(RawEvent::Keyboard(keys, true));

                    if keys.contains(Flags::RIGHT) {
                        let cnt = match window.fullscreen_state() {
                            FullscreenType::Off => FullscreenType::Desktop,
                            FullscreenType::True | FullscreenType::Desktop => FullscreenType::Off,
                        };
                        window.set_fullscreen(cnt)?;
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

    let program = init_shaders(&gl)?;

    // basic objects
    let land: Obj =
        load_obj(Cursor::new(include_str!("../objects/land.obj"))).map_err(Error::Obj)?;
    let cube: Obj =
        load_obj(Cursor::new(include_str!("../objects/cube.obj"))).map_err(Error::Obj)?;

    let timer = sdl.timer()?;

    let mut fps_counter = FPSCounter::new(&timer);
    fps_counter.set(144);

    let (fps_sender_1, fps_receiver_1) = bounded::<()>(1);
    let (fps_sender_2, fps_receiver_2) = bounded::<()>(1);

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

    // handle fps
    let _fps_timer = timer.add_timer(
        fps_counter.delay(),
        Box::new(|| _fps_timer_cb().unwrap_or_default()),
    );

    // real-time user input
    let (raw_event_sender, raw_event_receiver) = bounded::<RawEvent>(16);
    let (event_sender, event_receiver) = bounded::<GameEvent>(16);

    // container of objects
    // TODO - impl efficient graph eventually
    let mut objects: HashMap<u8, Object> = Default::default();
    objects.insert(
        0,
        Object::from_obj(&gl, program, land, [0.6, 0.7, 0.7, 1.0], 0)?,
    );

    let cam: Camera = Arc::new(RwLock::new(RawCamera::init(window.size())));
    let players: Players = Default::default();
    let running: Running = Arc::new(AtomicBool::new(true));

    let s = SyncSelect::default();

    handle_game_events(&s, event_receiver, ev.event_sender());

    render_loop(
        &s,
        (cam.clone(), players.clone(), running.clone()),
        (raw_event_receiver, event_sender.clone()),
        (fps_sender_1, fps_receiver_2),
        (fps_counter.reader(), Tps::default(), Ping::default()),
        cfg,
    );

    {
        let thread = s.thread();
        ctrlc::set_handler(move || {
            thread.unpark();
            if event_sender.send(GameEvent::Quit).is_err() {
                error!("Failed to notify event handler to quit")
            }
        })?;
    }

    _ = spawn(move || {
        s.join();
        error!("Fatal error.")
    });

    if let Err(e) = handle_raw_events(
        &gl,
        program,
        window,
        ep,
        (cam, players, &mut objects, running),
        raw_event_sender,
        cube,
    ) {
        error!("{}", e)
    }

    // clean everything up
    clean_up(&gl, program, objects.values());

    Ok(())
}
