// extensions/mod.rs
//
// Optional extension modules for ZapEngine.
// These are decoupled from core Entity/Scene — games opt-in by creating these systems.
//
// Clean architecture: core stays simple, complexity is additive.

pub mod easing;
pub mod transform;
pub mod tween;

pub use easing::{Easing, lerp, lerp_vec2, ease, ease_vec2};
pub use transform::{TransformGraph, LocalTransform};
pub use tween::{TweenState, Tween, TweenId, TweenTarget, TweenLoop};
