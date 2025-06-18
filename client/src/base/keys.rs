use crate::*;
use sdl2::keyboard::Scancode;

pub const fn from_scancode(key: Scancode) -> Keys {
    match key {
        Scancode::W => Keys::W,
        Scancode::A => Keys::A,
        Scancode::S => Keys::S,
        Scancode::D => Keys::D,

        Scancode::Up => Keys::UP,
        Scancode::Left => Keys::LEFT,
        Scancode::Down => Keys::DOWN,
        Scancode::Right => Keys::RIGHT,

        Scancode::Space => Keys::SPACE,
        Scancode::LShift | Scancode::RShift => Keys::SHIFT,
        Scancode::LCtrl | Scancode::RCtrl => Keys::CTRL,

        _ => Keys::empty(), // ignore everything else
    }
}

pub const fn try_from_scancode(key: Scancode) -> Option<Keys> {
    let kb = from_scancode(key);
    if kb.is_empty() { None } else { Some(kb) }
}
