use crossbeam_utils::Backoff;
use parking_lot::{Condvar, Mutex};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// sets the ready flag and notifies a waiting thread via condvar
#[derive(Clone)]
pub struct Notifier {
    ready: Arc<AtomicBool>,
    cvar: Arc<Condvar>,
}

/// sets the ready flag without notifying
#[derive(Clone)]
pub struct Setter {
    ready: Arc<AtomicBool>,
}

impl Setter {
    /// creates a new setter from a shared ready flag
    const fn new(ready: Arc<AtomicBool>) -> Self {
        Self { ready }
    }

    /// sets the ready flag to true
    pub fn set_ready(&self) {
        self.ready.store(true, Ordering::Release);
    }
}

/// allows checking if the ready flag is set
#[derive(Clone)]
pub struct Spectator {
    ready: Arc<AtomicBool>,
}

impl Spectator {
    /// creates a new spectator from a shared ready flag
    const fn new(ready: Arc<AtomicBool>) -> Self {
        Self { ready }
    }

    /// returns true if ready is set
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
}

impl Notifier {
    /// creates a new notifier from shared ready and condvar
    const fn new(ready: Arc<AtomicBool>, cvar: Arc<Condvar>) -> Self {
        Self { ready, cvar }
    }

    /// sets the ready flag and notifies one waiting thread
    pub fn notify(&self) {
        self.ready.store(true, Ordering::Release);
        self.cvar.notify_one();
    }
}

/// waits until ready is true using backoff and condvar
#[derive(Default)]
pub struct Waiter {
    ready: Arc<AtomicBool>,
    cvar: Arc<Condvar>,
    mutex: Mutex<()>,
}

impl Waiter {
    /// waits until the ready flag is true
    pub fn wait(&self) {
        let backoff = Backoff::new();

        loop {
            let is_ready = self.ready.load(Ordering::Acquire);
            if is_ready {
                break;
            }

            if backoff.is_completed() {
                let mut guard = self.mutex.lock();
                if !is_ready {
                    self.cvar.wait(&mut guard);
                }
            } else {
                backoff.snooze();
            }
        }
    }

    /// returns a notifier handle for setting and notifying
    pub fn notifier(&self) -> Notifier {
        Notifier::new(self.ready.clone(), self.cvar.clone())
    }

    /// returns a setter handle for setting only
    pub fn setter(&self) -> Setter {
        Setter::new(self.ready.clone())
    }

    /// returns a spectator handle for read-only access
    pub fn spectator(&self) -> Spectator {
        Spectator::new(self.ready.clone())
    }

    /// resets the ready flag to false
    pub fn reset(&self) {
        self.ready.store(false, Ordering::Release)
    }
}
