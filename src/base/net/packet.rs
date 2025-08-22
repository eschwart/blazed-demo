use crate::*;
use std::fmt::Debug;
use wopt::*;

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(id = 0)]
pub struct Ping;

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(id = 1)]
pub struct Flush;

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(derive(Clone, Copy, Debug, Default))]
#[wopt(id = 2)]
pub struct ClientHandshake;

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(derive(Clone, Copy, Debug, Default))]
#[wopt(id = 3)]
pub struct ServerHandshake(Id);

impl ServerHandshake {
    pub const fn new(id: Id) -> Self {
        Self(id)
    }

    pub const fn id(&self) -> Id {
        self.0
    }
}

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(derive(Clone, Copy, Debug, Default))]
#[wopt(id = 4)]
pub struct Keyboard {
    pub bits: u16,
    pub is_pressed: u8,
}

impl From<Keyboard> for Keys {
    fn from(value: Keyboard) -> Self {
        Keys::from_bits_retain(value.bits)
    }
}

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(derive(Clone, Copy, Debug, Default))]
#[wopt(id = 5)]
pub struct Wheel {
    pub precise_y: f32,
}

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(derive(Clone, Copy, Debug, Default))]
#[wopt(id = 6)]
pub struct Motion {
    pub xrel: i32,
    pub yrel: i32,
}

#[derive(Clone, Copy, Debug, WithOpt)]
#[wopt(derive(Clone, Copy, Debug, Default))]
#[wopt(id = 7)]
pub struct RemObj {
    pub id: Id,
}

#[derive(Clone, Copy, Debug, Default, WithOpt)]
#[wopt(derive(Clone, Copy, Debug, Default))]
#[wopt(id = 8)]
pub struct UptObj {
    #[wopt(required)]
    pub id: Id,
    pub kind: ObjType,
    pub dim: Vec3,
    pub color: Color,
    #[wopt(optional, serde)]
    pub cam: CameraAttr,
    #[wopt(required)]
    pub keys: Keys, // TODO - remove this
}
