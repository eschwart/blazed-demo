mod atom;
mod cam;
mod err;
mod keys;
mod net;
mod util;

pub use atom::*;
pub use cam::*;
pub use err::*;
pub use keys::*;
pub use net::*;
pub use util::*;

pub use crossbeam_utils::Backoff;
pub use log::{debug, error, info, trace, warn};
pub use parking_lot::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
pub use serde::{Deserialize, Serialize};
pub use spin_sleep::SpinSleeper;

pub fn init_logger() {
    std::env::set_var("RUST_LOG", "trace");
    env_logger::init();
}
