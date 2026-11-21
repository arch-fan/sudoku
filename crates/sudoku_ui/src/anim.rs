//! Easing curves and small animation helpers.
//!
//! GPUI animations are keyed by element id: changing the id restarts the
//! animation. We embed a monotonically increasing "generation" in ids so each
//! game event replays its effect exactly once.

use std::time::Duration;

use gpui::{Animation, SharedString};

pub fn ms(n: u64) -> Duration {
    Duration::from_millis(n.max(1))
}

pub fn id(prefix: &str, a: impl std::fmt::Display, generation: u64) -> SharedString {
    SharedString::from(format!("{prefix}-{a}-{generation}"))
}

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out_sine(t: f32) -> f32 {
    -((std::f32::consts::PI * t).cos() - 1.0) / 2.0
}

/// Overshoots then settles — great for "pop" effects.
pub fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.9;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Damped spring-ish oscillation ending at 1.
pub fn ease_out_elastic(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
    2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

/// 0 → 1 → 0 bump.
pub fn bump(t: f32) -> f32 {
    (std::f32::consts::PI * t.clamp(0.0, 1.0)).sin()
}

/// Horizontal shake offset (in "units", multiply by pixels).
pub fn shake(t: f32) -> f32 {
    (t * std::f32::consts::PI * 7.0).sin() * (1.0 - t).powf(1.5)
}

/// A wait followed by the actual animation, for staggered effects.
pub fn delayed(delay_ms: u64, duration_ms: u64, easing: fn(f32) -> f32) -> Vec<Animation> {
    vec![Animation::new(ms(delay_ms)), Animation::new(ms(duration_ms)).with_easing(easing)]
}
