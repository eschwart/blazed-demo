mod base;

pub use base::*;

use std::time::Duration;

pub const PACKET_SIZE: usize = 128;

// common delays
pub const SECOND: Duration = Duration::from_secs(1);
pub const MILISECOND: Duration = Duration::from_millis(1);
pub const PING_DELAY: Duration = Duration::from_millis(10);
