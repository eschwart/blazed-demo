use std::time::Duration;

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

/// the duration of a single interval based on the given rate.
pub fn tick_dur(value: impl Into<f32>) -> Duration {
    let f: f32 = value.into();

    // prevent panic
    if f.is_normal() {
        Duration::from_secs_f32(1.0 / f)
    } else {
        Duration::ZERO
    }
}
