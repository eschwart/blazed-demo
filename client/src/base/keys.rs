use crate::*;
use sdl2::keyboard::Scancode;

pub const fn from_scancode(key: Scancode) -> Flags {
    match key {
        Scancode::W => Flags::W,
        Scancode::A => Flags::A,
        Scancode::S => Flags::S,
        Scancode::D => Flags::D,

        Scancode::Up => Flags::UP,
        Scancode::Left => Flags::LEFT,
        Scancode::Down => Flags::DOWN,
        Scancode::Right => Flags::RIGHT,

        Scancode::Space => Flags::SPACE,
        Scancode::LShift | Scancode::RShift => Flags::SHIFT,
        Scancode::LCtrl | Scancode::RCtrl => Flags::CTRL,

        _ => Flags::empty(), // ignore everything else
    }
}

pub const fn try_from_scancode(key: Scancode) -> Option<Flags> {
    let flags = from_scancode(key);

    if flags.is_empty() {
        None
    } else {
        Some(flags)
    }
}
