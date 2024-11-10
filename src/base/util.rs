use std::{mem::size_of, slice::from_raw_parts};

#[inline(always)]
pub fn clamp_unchecked(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[inline(always)]
pub const fn cast_u16_slice(data: &[u16]) -> &[u8] {
    unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<u16>()) }
}
