mod cfg;
mod err;
mod fps;
mod keys;
mod net;
mod obj;
mod render;
mod util;

pub use cfg::*;
pub use err::*;
pub use fps::*;
pub use keys::*;
pub use net::*;
pub use obj::*;
pub use render::*;
pub use util::*;

pub use blazed_demo::*;

use std::{
    ops::Deref,
    sync::{atomic::AtomicU16, Arc},
    time::Duration,
};

/// An thread-safe read-write locked [`std::sync::atomic::AtomicU16`].
pub type Tps = Arc<AtomicU16>;

/// An thread-safe read-write locked [`blazed_demo::Flags`].
pub type Keys = Arc<RwLock<Flags>>;

/// An thread-safe read-write locked [`std::time::Duration`].
pub type Ping = Arc<RwLock<Duration>>;

/// A reference to a read-only locked [`RawObjects`].
pub type ObjectsRef<'a> = &'a RwLock<RawObjects>;

/// A convenience wrapper for a read-write locked [`RawObjects`].
#[derive(Clone, Debug)]
pub struct Objects {
    inner: Arc<RwLock<RawObjects>>,
}

impl Objects {
    pub fn new(raw: RawObjects) -> Self {
        let inner = Arc::new(RwLock::new(raw));
        Self { inner }
    }
}

impl Deref for Objects {
    type Target = RwLock<RawObjects>;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

/// A convenience wrapper for a read-write locked [`RawCamera`].
/// #[derive(Clone, Debug)]
pub struct Camera {
    inner: Arc<RwLock<RawCamera>>,
}

impl Camera {
    pub fn new(dims: (u32, u32)) -> Self {
        let inner = Arc::new(RwLock::new(RawCamera::new(dims)));
        Self { inner }
    }
}

impl Deref for Camera {
    type Target = RwLock<RawCamera>;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

/// Events related to rendering.
#[derive(Clone, Copy, Debug)]
pub enum RenderAction {
    AspectRatio { w: i32, h: i32 },
    Flush,
}

/// Events related to objects.
#[derive(Clone, Copy, Debug)]
pub enum ObjectAction {
    Add { data: ObjectData },
    Rem { id: Id },
    Upt { data: ObjectData },
    User { data: PlayerData },
}

/// Event wrappers related to the user.
#[derive(Clone, Copy, Debug)]
pub enum UserAction {
    Input(Input),
}

/// Event wrappers related to the game.
#[derive(Clone, Copy, Debug)]
pub enum GameEvent {
    Quit,
    Reset,
    Render(RenderAction),
    Object(ObjectAction),
    User(UserAction),
}

/// Event wrappers related to the backend.
#[derive(Clone, Copy, Debug)]
pub enum RawEvent {
    Quit,
    MouseWheel(f32),
    MouseMotion(i32, i32),
    Keyboard(Flags, bool),
    AspectRatio(i32, i32),
}
