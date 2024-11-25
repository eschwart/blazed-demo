mod cfg;
mod err;
mod fps;
mod keys;
mod net;
mod obj;
mod render;
mod util;

use std::{
    collections::HashMap,
    sync::{atomic::AtomicU16, Arc},
    time::Duration,
};

pub use cfg::*;
pub use err::*;
pub use fps::*;
pub use keys::*;
pub use net::*;
pub use obj::*;
pub use render::*;
pub use util::*;

pub use blazed_demo::{Camera as RawCamera, *};

pub type Tps = Arc<AtomicU16>;
pub type Keys = Arc<RwLock<Flags>>;
pub type Ping = Arc<RwLock<Duration>>;
pub type Camera = Arc<RwLock<RawCamera>>;
pub type Players = Arc<RwLock<HashMap<u8, Player>>>;

#[derive(Debug)]
pub enum ObjectAction {
    Add(Player),
    Remove(u8),
}

#[derive(Debug)]
pub enum RawEvent {
    Quit,
    MouseWheel(f32),
    MouseMotion(i32, i32),
    Keyboard(Flags, bool),
    AspectRatio(i32, i32),
}

#[derive(Debug)]
pub enum GameEvent {
    Quit,
    Reset,
    Render,
    Object(ObjectAction),
}
