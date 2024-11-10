mod cam;
mod err;
mod keys;
mod net;
mod util;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub use cam::*;
pub use err::*;
pub use keys::*;
pub use net::*;
pub use util::*;

pub use clap;
pub use crossbeam_channel;
pub use crossbeam_utils::Backoff;
pub use ctrlc;
pub use log::{debug, error, info, trace, warn};
pub use packet_enum;
pub use parking_lot;
pub use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use serde::{Deserialize, Serialize};
pub use spin_sleep::SpinSleeper;
pub use strum::Display;
pub use sync_select::*;

pub type Running = Arc<AtomicBool>;

pub fn init_logger() {
    std::env::set_var("RUST_LOG", "trace");
    env_logger::init();
}
