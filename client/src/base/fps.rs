use crate::*;
use sdl2::TimerSubsystem;
use std::{
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
    thread::{sleep, spawn, JoinHandle},
    time::Duration,
};

pub struct Counter(u64);

#[derive(Clone, Default)]
pub struct Fps {
    value: Arc<AtomicU16>,   // frames-per-second
    counter: Arc<AtomicU16>, // frame incrementer
}

impl Fps {
    pub fn get(&self) -> u16 {
        self.value.load(Ordering::SeqCst)
    }
}

pub struct FPSCounter {
    spin: SpinSleeper,
    limit: Duration,
    fps: Fps,
    freq: f32,
    _handle: JoinHandle<()>,
}

impl FPSCounter {
    pub fn new(timer: &TimerSubsystem) -> Self {
        let spin = SpinSleeper::default();
        let limit = Self::limit_dur(60);

        let freq = timer.performance_frequency() as f32;

        let fps: Fps = Default::default();

        let _handle = {
            let value_thread = fps.value.clone();
            let counter_thread = fps.counter.clone();

            spawn(move || loop {
                sleep(SECOND);
                let value = counter_thread.swap(0, Ordering::Relaxed);
                value_thread.swap(value, Ordering::Relaxed);
            })
        };

        Self {
            spin,
            limit,
            fps,
            freq,
            _handle,
        }
    }

    pub const fn delay(&self) -> u32 {
        self.limit.as_millis() as u32
    }

    pub fn reader(&self) -> Fps {
        self.fps.clone()
    }

    pub fn set(&mut self, fps: u16) {
        self.limit = Self::limit_dur(fps);
    }

    pub fn start(&self, timer: &TimerSubsystem) -> Counter {
        Counter(timer.performance_counter())
    }

    pub fn stop(&self, timer: &TimerSubsystem, start: Counter) {
        let end = timer.performance_counter();

        let elapsed_sec = (end - start.0) as f32 / self.freq;
        let elapsed_dur = Duration::from_secs_f32(elapsed_sec);

        if elapsed_dur < self.limit {
            let dif = self.limit - elapsed_dur;
            self.spin.sleep(dif)
        }
        self.fps.counter.fetch_add(1, Ordering::Relaxed);
    }

    fn limit_dur(fps: u16) -> Duration {
        Duration::from_secs_f32((1000.0 / fps as f32) * 0.001)
    }
}
