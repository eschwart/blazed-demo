mod base;

pub use base::*;

use atomint::*;
use std::time::Duration;
use ultraviolet::Vec3;

pub type Id = i8;
pub type AtomicId = <Id as AtomicInt>::Atomic;

// TODO - tune this
pub const PACKET_SIZE: usize = 1024;

// default dynamic ports (arbitrary, for now)
pub const TCP_PORT: u16 = 54269;
pub const UDP_PORT: u16 = 54277;

// common delays
pub const SECOND: Duration = Duration::from_secs(1);
pub const MILISECOND: Duration = Duration::from_millis(1);

// platform rates
pub const GAME_SPEED: Duration = Duration::from_millis(3);
pub const PING_MINIMUM: Duration = Duration::from_millis(10);

// common mathematical values
pub const RADIAN: f32 = std::f32::consts::PI / 180.0;

// unit vectors of each axis as basic vectors
pub const X_AXIS: Vec3 = Vec3::new(1.0, 0.0, 0.0);
pub const Y_AXIS: Vec3 = Vec3::new(0.0, 1.0, 0.0);
pub const Z_AXIS: Vec3 = Vec3::new(0.0, 0.0, 1.0);

// diagonal vector as a basic and unit vector
pub const DIAGONAL: Vec3 = Vec3::new(1.0, 1.0, 1.0);
