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

use atomic_enum::*;
use std::{ops::Deref, sync::Arc};

/// A reference to a read-only locked [`RawObjects`].
pub type ObjectsRef<'a> = &'a RwLock<RawObjects>;

/// A thread-safe [`RawRenderState`].
pub type RenderState = Arc<AtomicRenderStateKind>;

/// Current state of the rendering thread.
#[atomic_enum]
pub enum RenderStateKind {
    Pass,
    Reload,
    Quit,
}

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
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub enum ObjectAction {
    Add { data: UptObj },
    Upt { data: UptObjOpt },
    Remove { id: Id },
    User { data: UptObjOpt },
}

/// Event wrappers related to the user.
#[derive(Clone, Copy, Debug)]
pub enum UserAction {
    Keyboard(Keys),
    Wheel(Wheel),
    Motion(Motion),
}

/// Event wrappers related to the game.
#[derive(Clone, Debug)]
pub enum GameEvent {
    Quit, // exit the program
    Reset,
    Render(RenderAction),
    Object(ObjectAction),
    User(UserAction),
    Fps(Id), // dynamically update FPS limit
}

/// Event wrappers related to the backend.
#[derive(Clone, Copy, Debug)]
pub enum RawEvent {
    Quit,
    MouseWheel(f32),
    MouseMotion(i32, i32),
    Keyboard(Keys, bool),
    AspectRatio(i32, i32),
}
