use atomflag::*;
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum KeyState {
    #[default]
    Idle, // in the UI or something
    Player, // actively playing
    Typing, // typing (e.g., chat)
}

bitflags! {
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Pod, Zeroable, AtomicFlag)]
    #[atomic_flag(wrapper = "Arc")]
    pub struct Keys: u16 {
        const W     = 0b_00000000_00000001; // FORWARD
        const A     = 0b_00000000_00000010; // LEFT
        const S     = 0b_00000000_00000100; // BACK
        const D     = 0b_00000000_00001000; // RIGHT

        const UP    = 0b_00000000_00010000; // UP
        const LEFT  = 0b_00000000_00100000; // LEFT
        const DOWN  = 0b_00000000_01000000; // DOWN
        const RIGHT = 0b_00000000_10000000; // RIGHT

        const SPACE = 0b_00000001_00000000; // UP
        const SHIFT = 0b_00000010_00000000; // DOWN
        const CTRL  = 0b_00000100_00000000; // CROUCH
    }

    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Pod, Zeroable)]
    pub struct ObjType: u8 {
        const Basic  = 0b_0000_0001;
        const Player = 0b_0000_0010;
    }
}

impl Keys {
    const NORMAL: Self = Self::UP
        .union(Self::LEFT)
        .union(Self::DOWN)
        .union(Self::RIGHT)
        .union(Self::CTRL);

    const CONTINUOUS: Self = Self::W
        .union(Self::A)
        .union(Self::S)
        .union(Self::D)
        .union(Self::SPACE)
        .union(Self::SHIFT);

    // check if keys are all normal
    pub const fn is_normal(self) -> bool {
        Self::NORMAL.contains(self) && !Self::CONTINUOUS.contains(self)
    }

    // check if keys are all continuous
    pub const fn is_continuous(self, state: KeyState) -> bool {
        match state {
            KeyState::Player => Self::CONTINUOUS.contains(self) && !Self::NORMAL.contains(self),
            _ => false,
        }
    }
}
