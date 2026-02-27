// extensions/tween.rs
//
// Tween system — manages animated value transitions by EntityId.
// Completely decoupled from Entity/Scene internals.
//
// Usage:
//   let mut tweens = TweenState::new();
//   tweens.add(entity_id, Tween::position(from, to, 0.5, Easing::QuadOut));
//   tweens.tick(dt, &mut scene);  // Advances all tweens, updates entities

use std::collections::HashMap;
use glam::Vec2;
use crate::api::types::EntityId;
use crate::core::scene::Scene;
use super::easing::{Easing, ease, ease_vec2};

/// What property a tween animates.
#[derive(Debug, Clone, Copy)]
pub enum TweenTarget {
    /// Animate Entity.pos
    Position { from: Vec2, to: Vec2 },
    /// Animate Entity.pos.x only
    PositionX { from: f32, to: f32 },
    /// Animate Entity.pos.y only
    PositionY { from: f32, to: f32 },
    /// Animate Entity.rotation
    Rotation { from: f32, to: f32 },
    /// Animate Entity.scale (uniform)
    Scale { from: Vec2, to: Vec2 },
    /// Animate Entity.scale.x only
    ScaleX { from: f32, to: f32 },
    /// Animate Entity.scale.y only
    ScaleY { from: f32, to: f32 },
    /// Animate sprite alpha (if sprite exists)
    Alpha { from: f32, to: f32 },
}

/// What happens when a tween completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TweenLoop {
    /// Stop and remove the tween.
    #[default]
    Once,
    /// Restart from the beginning.
    Loop,
    /// Reverse direction (ping-pong).
    PingPong,
}

/// A single tween animation.
#[derive(Debug, Clone)]
pub struct Tween {
    /// What to animate.
    pub target: TweenTarget,
    /// Duration in seconds.
    pub duration: f32,
    /// Elapsed time.
    pub elapsed: f32,
    /// Easing function.
    pub easing: Easing,
    /// Loop behavior.
    pub loop_mode: TweenLoop,
    /// Whether currently playing (can be paused).
    pub playing: bool,
    /// For ping-pong: current direction (true = forward).
    forward: bool,
    /// Optional callback ID to emit as GameEvent when complete.
    pub on_complete: Option<u32>,
}

impl Tween {
    /// Create a position tween.
    pub fn position(from: Vec2, to: Vec2, duration: f32, easing: Easing) -> Self {
        Self {
            target: TweenTarget::Position { from, to },
            duration,
            elapsed: 0.0,
            easing,
            loop_mode: TweenLoop::Once,
            playing: true,
            forward: true,
            on_complete: None,
        }
    }

    /// Create a position X tween.
    pub fn position_x(from: f32, to: f32, duration: f32, easing: Easing) -> Self {
        Self {
            target: TweenTarget::PositionX { from, to },
            duration,
            elapsed: 0.0,
            easing,
            loop_mode: TweenLoop::Once,
            playing: true,
            forward: true,
            on_complete: None,
        }
    }

    /// Create a position Y tween.
    pub fn position_y(from: f32, to: f32, duration: f32, easing: Easing) -> Self {
        Self {
            target: TweenTarget::PositionY { from, to },
            duration,
            elapsed: 0.0,
            easing,
            loop_mode: TweenLoop::Once,
            playing: true,
            forward: true,
            on_complete: None,
        }
    }

    /// Create a rotation tween.
    pub fn rotation(from: f32, to: f32, duration: f32, easing: Easing) -> Self {
        Self {
            target: TweenTarget::Rotation { from, to },
            duration,
            elapsed: 0.0,
            easing,
            loop_mode: TweenLoop::Once,
            playing: true,
            forward: true,
            on_complete: None,
        }
    }

    /// Create a scale tween.
    pub fn scale(from: Vec2, to: Vec2, duration: f32, easing: Easing) -> Self {
        Self {
            target: TweenTarget::Scale { from, to },
            duration,
            elapsed: 0.0,
            easing,
            loop_mode: TweenLoop::Once,
            playing: true,
            forward: true,
            on_complete: None,
        }
    }

    /// Create a uniform scale tween.
    pub fn scale_uniform(from: f32, to: f32, duration: f32, easing: Easing) -> Self {
        Self::scale(Vec2::splat(from), Vec2::splat(to), duration, easing)
    }

    /// Create an alpha (fade) tween.
    pub fn alpha(from: f32, to: f32, duration: f32, easing: Easing) -> Self {
        Self {
            target: TweenTarget::Alpha { from, to },
            duration,
            elapsed: 0.0,
            easing,
            loop_mode: TweenLoop::Once,
            playing: true,
            forward: true,
            on_complete: None,
        }
    }

    /// Fade in from transparent.
    pub fn fade_in(duration: f32, easing: Easing) -> Self {
        Self::alpha(0.0, 1.0, duration, easing)
    }

    /// Fade out to transparent.
    pub fn fade_out(duration: f32, easing: Easing) -> Self {
        Self::alpha(1.0, 0.0, duration, easing)
    }

    // -- Builder methods --

    pub fn with_loop(mut self, mode: TweenLoop) -> Self {
        self.loop_mode = mode;
        self
    }

    pub fn with_on_complete(mut self, event_id: u32) -> Self {
        self.on_complete = Some(event_id);
        self
    }

    pub fn paused(mut self) -> Self {
        self.playing = false;
        self
    }

    /// Normalized progress [0, 1].
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        }
    }

    /// Whether the tween has completed (for Once mode).
    pub fn is_complete(&self) -> bool {
        self.loop_mode == TweenLoop::Once && self.elapsed >= self.duration
    }
}

/// Handle to a tween for later reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TweenId(pub u32);

/// Manages all active tweens.
#[derive(Debug, Default)]
pub struct TweenState {
    tweens: HashMap<TweenId, (EntityId, Tween)>,
    next_id: u32,
    /// Completed tween events to be polled.
    completed_events: Vec<u32>,
}

impl TweenState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tween for an entity. Returns a handle for later control.
    pub fn add(&mut self, entity: EntityId, tween: Tween) -> TweenId {
        let id = TweenId(self.next_id);
        self.next_id += 1;
        self.tweens.insert(id, (entity, tween));
        id
    }

    /// Remove a tween by handle.
    pub fn remove(&mut self, id: TweenId) -> bool {
        self.tweens.remove(&id).is_some()
    }

    /// Remove all tweens for an entity.
    pub fn remove_entity(&mut self, entity: EntityId) {
        self.tweens.retain(|_, (e, _)| *e != entity);
    }

    /// Pause a tween.
    pub fn pause(&mut self, id: TweenId) {
        if let Some((_, tween)) = self.tweens.get_mut(&id) {
            tween.playing = false;
        }
    }

    /// Resume a paused tween.
    pub fn resume(&mut self, id: TweenId) {
        if let Some((_, tween)) = self.tweens.get_mut(&id) {
            tween.playing = true;
        }
    }

    /// Pause all tweens.
    pub fn pause_all(&mut self) {
        for (_, tween) in self.tweens.values_mut() {
            tween.playing = false;
        }
    }

    /// Resume all tweens.
    pub fn resume_all(&mut self) {
        for (_, tween) in self.tweens.values_mut() {
            tween.playing = true;
        }
    }

    /// Get a tween by handle.
    pub fn get(&self, id: TweenId) -> Option<&Tween> {
        self.tweens.get(&id).map(|(_, t)| t)
    }

    /// Get a tween mutably.
    pub fn get_mut(&mut self, id: TweenId) -> Option<&mut Tween> {
        self.tweens.get_mut(&id).map(|(_, t)| t)
    }

    /// Advance all tweens and apply to entities in the scene.
    /// Returns the number of tweens that completed this tick.
    pub fn tick(&mut self, dt: f32, scene: &mut Scene) -> usize {
        let mut completed = Vec::new();

        for (&id, (entity_id, tween)) in self.tweens.iter_mut() {
            if !tween.playing {
                continue;
            }

            // Advance time
            tween.elapsed += dt;

            // Calculate progress
            let raw_t = if tween.duration > 0.0 {
                tween.elapsed / tween.duration
            } else {
                1.0
            };

            let t = if tween.forward {
                raw_t.clamp(0.0, 1.0)
            } else {
                (1.0 - raw_t).clamp(0.0, 1.0)
            };

            // Apply to entity
            if let Some(entity) = scene.get_mut(*entity_id) {
                match tween.target {
                    TweenTarget::Position { from, to } => {
                        entity.pos = ease_vec2(from, to, t, tween.easing);
                    }
                    TweenTarget::PositionX { from, to } => {
                        entity.pos.x = ease(from, to, t, tween.easing);
                    }
                    TweenTarget::PositionY { from, to } => {
                        entity.pos.y = ease(from, to, t, tween.easing);
                    }
                    TweenTarget::Rotation { from, to } => {
                        entity.rotation = ease(from, to, t, tween.easing);
                    }
                    TweenTarget::Scale { from, to } => {
                        entity.scale = ease_vec2(from, to, t, tween.easing);
                    }
                    TweenTarget::ScaleX { from, to } => {
                        entity.scale.x = ease(from, to, t, tween.easing);
                    }
                    TweenTarget::ScaleY { from, to } => {
                        entity.scale.y = ease(from, to, t, tween.easing);
                    }
                    TweenTarget::Alpha { from, to } => {
                        if let Some(sprite) = &mut entity.sprite {
                            sprite.alpha = ease(from, to, t, tween.easing);
                        }
                    }
                }
            }

            // Handle completion
            if tween.elapsed >= tween.duration {
                match tween.loop_mode {
                    TweenLoop::Once => {
                        if let Some(event_id) = tween.on_complete {
                            self.completed_events.push(event_id);
                        }
                        completed.push(id);
                    }
                    TweenLoop::Loop => {
                        tween.elapsed = 0.0;
                    }
                    TweenLoop::PingPong => {
                        tween.elapsed = 0.0;
                        tween.forward = !tween.forward;
                    }
                }
            }
        }

        let count = completed.len();
        for id in completed {
            self.tweens.remove(&id);
        }

        count
    }

    /// Drain completed tween events (for GameEvent emission).
    pub fn drain_completed(&mut self) -> impl Iterator<Item = u32> + '_ {
        self.completed_events.drain(..)
    }

    /// Number of active tweens.
    pub fn len(&self) -> usize {
        self.tweens.len()
    }

    /// Whether there are no active tweens.
    pub fn is_empty(&self) -> bool {
        self.tweens.is_empty()
    }

    /// Clear all tweens.
    pub fn clear(&mut self) {
        self.tweens.clear();
        self.completed_events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::entity::Entity;

    #[test]
    fn tween_position() {
        let mut tweens = TweenState::new();
        let mut scene = Scene::new();
        let id = EntityId(1);

        scene.spawn(Entity::new(id).with_pos(Vec2::ZERO));
        tweens.add(id, Tween::position(
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            1.0,
            Easing::Linear,
        ));

        // Tick halfway
        tweens.tick(0.5, &mut scene);
        let e = scene.get(id).unwrap();
        assert!((e.pos.x - 50.0).abs() < 0.01);

        // Tick to completion
        tweens.tick(0.5, &mut scene);
        let e = scene.get(id).unwrap();
        assert!((e.pos.x - 100.0).abs() < 0.01);

        // Tween should be removed
        assert!(tweens.is_empty());
    }

    #[test]
    fn tween_loop() {
        let mut tweens = TweenState::new();
        let mut scene = Scene::new();
        let id = EntityId(1);

        scene.spawn(Entity::new(id));
        tweens.add(id, Tween::position(
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            1.0,
            Easing::Linear,
        ).with_loop(TweenLoop::Loop));

        // Complete one cycle
        tweens.tick(1.0, &mut scene);

        // Tween should still exist
        assert_eq!(tweens.len(), 1);
    }

    #[test]
    fn tween_ping_pong() {
        let mut tweens = TweenState::new();
        let mut scene = Scene::new();
        let id = EntityId(1);

        scene.spawn(Entity::new(id).with_pos(Vec2::ZERO));
        tweens.add(id, Tween::position(
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            1.0,
            Easing::Linear,
        ).with_loop(TweenLoop::PingPong));

        // Go to end
        tweens.tick(1.0, &mut scene);
        let e = scene.get(id).unwrap();
        assert!((e.pos.x - 100.0).abs() < 0.01);

        // Go back to start
        tweens.tick(1.0, &mut scene);
        let e = scene.get(id).unwrap();
        assert!((e.pos.x - 0.0).abs() < 0.01);
    }

    #[test]
    fn remove_entity_tweens() {
        let mut tweens = TweenState::new();
        let id = EntityId(1);

        tweens.add(id, Tween::position(Vec2::ZERO, Vec2::ONE, 1.0, Easing::Linear));
        tweens.add(id, Tween::rotation(0.0, 1.0, 1.0, Easing::Linear));

        assert_eq!(tweens.len(), 2);
        tweens.remove_entity(id);
        assert!(tweens.is_empty());
    }
}
