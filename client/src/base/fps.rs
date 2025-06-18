use crate::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::Duration,
};

#[derive(Debug, Default)]
pub struct Fps {
    inner: AtomicU8,  // frames-per-second
    live: AtomicU8,   // frame incrementer
    target: AtomicU8, // frame target [0 for max]
}

impl Fps {
    pub fn get(&self) -> u8 {
        self.inner.load(Ordering::SeqCst)
    }

    pub fn incr(&self) {
        let _ = self
            .live
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                if val < u8::MAX {
                    Some(val + 1)
                } else {
                    Some(u8::MAX)
                }
            });
    }

    pub fn target(&self) -> u8 {
        self.target.load(Ordering::SeqCst)
    }

    pub fn swap_target(&self, fps: u8) -> u8 {
        self.target.swap(fps, Ordering::Relaxed)
    }

    pub fn reset(&self) {
        let fps = self.live.swap(0, Ordering::Relaxed);
        self.inner.swap(fps, Ordering::Relaxed);
    }
}

pub type RawFps = Arc<Fps>;

#[derive(Clone, Debug, Default)]
pub struct Limit {
    inner: RawRate,
}

impl Limit {
    pub fn get(&self) -> Duration {
        *self.inner.read()
    }

    pub fn set(&self, fps: u8) {
        *self.inner.write() = tick_dur(fps)
    }
}
