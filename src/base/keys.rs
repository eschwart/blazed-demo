use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Flags: u16 {
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
}
