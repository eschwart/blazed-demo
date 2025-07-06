use crate::*;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

#[derive(Debug, Default)]
pub struct Fps {
    inner: AtomicId,  // frames-per-second
    live: AtomicId,   // frame incrementer
    target: AtomicId, // frame target [0 for max]
}

impl Fps {
    pub fn get(&self) -> Id {
        self.inner.load(Ordering::SeqCst)
    }

    pub fn incr(&self) {
        let _ = self
            .live
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
                if val < Id::MAX {
                    Some(val + 1)
                } else {
                    Some(Id::MAX)
                }
            });
    }

    pub fn target(&self) -> Id {
        self.target.load(Ordering::SeqCst)
    }

    pub fn swap_target(&self, fps: Id) -> Id {
        self.target.swap(fps, Ordering::Relaxed)
    }

    pub fn reset(&self) {
        let fps = self.live.swap(0, Ordering::Relaxed);
        self.inner.swap(fps, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Limit {
    inner: Arc<RwLock<Duration>>,
}

impl Limit {
    pub fn get(&self) -> Duration {
        *self.inner.read()
    }

    pub fn set(&self, fps: Id) {
        *self.inner.write() = tick_dur(fps)
    }
}
