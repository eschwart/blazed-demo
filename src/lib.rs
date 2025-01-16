mod base;

pub use base::*;

use nalgebra::Unit;
use std::time::Duration;

pub type Id = i8;
pub type AtomicId = <Id as AtomicInt>::Atomic;

// default dynamic ports (arbitrary)
pub const TCP_PORT: u16 = 54269;
pub const UDP_PORT: u16 = 54277;

// default packet size (sort of arbitrary for now)
pub const PACKET_SIZE: usize = 128;

// common delays
pub const SECOND: Duration = Duration::from_secs(1);
pub const MILISECOND: Duration = Duration::from_millis(1);

// platform rates
pub const TICK_RATE: Duration = Duration::from_millis(4);
pub const PING_MINIMUM: Duration = Duration::from_millis(10);

// common mathematical values
pub const RADIAN: f32 = std::f32::consts::PI / 180.0;

// unit vectors of each axis as basic vectors
pub const X_AXIS: Vector = Vector::new(1.0, 0.0, 0.0);
pub const Y_AXIS: Vector = Vector::new(0.0, 1.0, 0.0);
pub const Z_AXIS: Vector = Vector::new(0.0, 0.0, 1.0);

// unit vectors of each axis as unit vectors
pub const X_AXIS_UNIT: Unit<Vector> = Unit::new_unchecked(X_AXIS);
pub const Y_AXIS_UNIT: Unit<Vector> = Unit::new_unchecked(Y_AXIS);
pub const Z_AXIS_UNIT: Unit<Vector> = Unit::new_unchecked(Z_AXIS);

// diagonal vector as a basic and unit vector
pub const DIAGONAL: Vector = Vector::new(1.0, 1.0, 1.0);
pub const DIAGONAL_UNIT: Unit<Vector> = Unit::new_unchecked(DIAGONAL);
