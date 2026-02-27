// extensions/easing.rs
//
// Pure easing functions for animation interpolation.
// No dependencies on Entity/Scene — just math.

use std::f32::consts::PI;

/// Easing function type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// Constant velocity (no easing).
    #[default]
    Linear,
    /// Slow start.
    QuadIn,
    /// Slow end.
    QuadOut,
    /// Slow start and end.
    QuadInOut,
    /// Stronger slow start.
    CubicIn,
    /// Stronger slow end.
    CubicOut,
    /// Stronger slow start and end.
    CubicInOut,
    /// Very strong slow start.
    QuartIn,
    /// Very strong slow end.
    QuartOut,
    /// Very strong slow start and end.
    QuartInOut,
    /// Sine wave easing (smooth).
    SineIn,
    SineOut,
    SineInOut,
    /// Exponential easing (dramatic).
    ExpoIn,
    ExpoOut,
    ExpoInOut,
    /// Overshoot then settle.
    BackIn,
    BackOut,
    BackInOut,
    /// Bouncy finish.
    BounceOut,
    /// Elastic spring.
    ElasticOut,
}

impl Easing {
    /// Apply the easing function to a normalized time value `t` in [0, 1].
    /// Returns the eased value, also typically in [0, 1] (but can overshoot for Back/Elastic).
    #[inline]
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,

            // Quadratic
            Easing::QuadIn => t * t,
            Easing::QuadOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }

            // Cubic
            Easing::CubicIn => t * t * t,
            Easing::CubicOut => 1.0 - (1.0 - t).powi(3),
            Easing::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }

            // Quartic
            Easing::QuartIn => t * t * t * t,
            Easing::QuartOut => 1.0 - (1.0 - t).powi(4),
            Easing::QuartInOut => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }

            // Sine
            Easing::SineIn => 1.0 - (t * PI / 2.0).cos(),
            Easing::SineOut => (t * PI / 2.0).sin(),
            Easing::SineInOut => -((PI * t).cos() - 1.0) / 2.0,

            // Exponential
            Easing::ExpoIn => {
                if t == 0.0 { 0.0 } else { 2.0_f32.powf(10.0 * t - 10.0) }
            }
            Easing::ExpoOut => {
                if t == 1.0 { 1.0 } else { 1.0 - 2.0_f32.powf(-10.0 * t) }
            }
            Easing::ExpoInOut => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2.0_f32.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
                }
            }

            // Back (overshoot)
            Easing::BackIn => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                C3 * t * t * t - C1 * t * t
            }
            Easing::BackOut => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
            }
            Easing::BackInOut => {
                const C1: f32 = 1.70158;
                const C2: f32 = C1 * 1.525;
                if t < 0.5 {
                    (2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2) + 2.0) / 2.0
                }
            }

            // Bounce
            Easing::BounceOut => bounce_out(t),

            // Elastic
            Easing::ElasticOut => {
                const C4: f32 = (2.0 * PI) / 3.0;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
                }
            }
        }
    }
}

#[inline]
fn bounce_out(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;

    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

// ── Interpolation helpers ────────────────────────────────────────────────

/// Linearly interpolate between two values.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Linearly interpolate between two Vec2 values.
#[inline]
pub fn lerp_vec2(a: glam::Vec2, b: glam::Vec2, t: f32) -> glam::Vec2 {
    a + (b - a) * t
}

/// Interpolate with easing.
#[inline]
pub fn ease(a: f32, b: f32, t: f32, easing: Easing) -> f32 {
    lerp(a, b, easing.apply(t))
}

/// Interpolate Vec2 with easing.
#[inline]
pub fn ease_vec2(a: glam::Vec2, b: glam::Vec2, t: f32, easing: Easing) -> glam::Vec2 {
    lerp_vec2(a, b, easing.apply(t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_endpoints() {
        assert_eq!(Easing::Linear.apply(0.0), 0.0);
        assert_eq!(Easing::Linear.apply(1.0), 1.0);
        assert_eq!(Easing::Linear.apply(0.5), 0.5);
    }

    #[test]
    fn quad_out_faster_start() {
        // QuadOut should be > 0.5 at t=0.5 (faster start, slower end)
        let mid = Easing::QuadOut.apply(0.5);
        assert!(mid > 0.5, "QuadOut at 0.5 should be > 0.5, got {}", mid);
    }

    #[test]
    fn back_overshoots() {
        // BackOut should overshoot slightly
        let early = Easing::BackOut.apply(0.3);
        assert!(early > 0.3, "BackOut should overshoot");
    }

    #[test]
    fn ease_interpolates() {
        let result = ease(100.0, 200.0, 0.5, Easing::Linear);
        assert!((result - 150.0).abs() < 0.001);
    }
}
