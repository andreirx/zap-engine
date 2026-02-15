//! Animation component for sprite frame sequences.
//!
//! Provides automatic sprite animation by cycling through frames in an atlas.

use std::collections::HashMap;

/// Definition of a single animation sequence.
#[derive(Debug, Clone)]
pub struct AnimationDef {
    /// Frame indices as (col, row) pairs in the atlas.
    pub frames: Vec<(f32, f32)>,
    /// Seconds per frame.
    pub frame_duration: f32,
    /// Whether to loop when reaching the end.
    pub looping: bool,
}

impl AnimationDef {
    /// Create a horizontal strip animation (consecutive columns, same row).
    pub fn horizontal_strip(row: f32, start_col: f32, frame_count: u32, fps: f32) -> Self {
        let frames: Vec<(f32, f32)> = (0..frame_count)
            .map(|i| (start_col + i as f32, row))
            .collect();
        Self {
            frames,
            frame_duration: 1.0 / fps,
            looping: true,
        }
    }

    /// Create a vertical strip animation (consecutive rows, same column).
    pub fn vertical_strip(col: f32, start_row: f32, frame_count: u32, fps: f32) -> Self {
        let frames: Vec<(f32, f32)> = (0..frame_count)
            .map(|i| (col, start_row + i as f32))
            .collect();
        Self {
            frames,
            frame_duration: 1.0 / fps,
            looping: true,
        }
    }

    /// Create from explicit frame list.
    pub fn from_frames(frames: Vec<(f32, f32)>, fps: f32, looping: bool) -> Self {
        Self {
            frames,
            frame_duration: 1.0 / fps,
            looping,
        }
    }

    /// Get the number of frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get the total duration of the animation.
    pub fn total_duration(&self) -> f32 {
        self.frame_duration * self.frames.len() as f32
    }
}

/// Animation state for an entity.
#[derive(Debug, Clone)]
pub struct AnimationComponent {
    /// Named animations available for this entity.
    pub animations: HashMap<String, AnimationDef>,
    /// Currently playing animation name.
    pub current: String,
    /// Current frame index.
    pub frame_index: usize,
    /// Time accumulated in current frame.
    pub frame_timer: f32,
    /// Whether animation is playing (vs paused).
    pub playing: bool,
    /// Playback speed multiplier (1.0 = normal).
    pub speed: f32,
}

impl Default for AnimationComponent {
    fn default() -> Self {
        Self {
            animations: HashMap::new(),
            current: String::new(),
            frame_index: 0,
            frame_timer: 0.0,
            playing: true,
            speed: 1.0,
        }
    }
}

impl AnimationComponent {
    /// Create with a set of named animations.
    pub fn new(animations: HashMap<String, AnimationDef>) -> Self {
        let current = animations.keys().next().cloned().unwrap_or_default();
        Self {
            animations,
            current,
            ..Default::default()
        }
    }

    /// Create with a single default animation.
    pub fn single(name: impl Into<String>, def: AnimationDef) -> Self {
        let name = name.into();
        let mut animations = HashMap::new();
        animations.insert(name.clone(), def);
        Self {
            animations,
            current: name,
            ..Default::default()
        }
    }

    /// Add an animation.
    pub fn add(&mut self, name: impl Into<String>, def: AnimationDef) {
        self.animations.insert(name.into(), def);
    }

    /// Play a named animation from the beginning.
    pub fn play(&mut self, name: &str) {
        if self.animations.contains_key(name) {
            self.current = name.to_string();
            self.frame_index = 0;
            self.frame_timer = 0.0;
            self.playing = true;
        }
    }

    /// Play animation only if it's different from current.
    pub fn play_if_different(&mut self, name: &str) {
        if self.current != name {
            self.play(name);
        }
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Resume playback.
    pub fn resume(&mut self) {
        self.playing = true;
    }

    /// Stop and reset to frame 0.
    pub fn stop(&mut self) {
        self.playing = false;
        self.frame_index = 0;
        self.frame_timer = 0.0;
    }

    /// Get current animation definition.
    pub fn current_def(&self) -> Option<&AnimationDef> {
        self.animations.get(&self.current)
    }

    /// Get current frame (col, row) for sprite rendering.
    pub fn current_frame(&self) -> Option<(f32, f32)> {
        self.current_def()
            .and_then(|def| def.frames.get(self.frame_index).copied())
    }

    /// Check if animation has finished (only meaningful for non-looping).
    pub fn is_finished(&self) -> bool {
        if let Some(def) = self.current_def() {
            !def.looping && self.frame_index >= def.frames.len().saturating_sub(1)
        } else {
            true
        }
    }

    /// Advance animation by dt seconds. Returns true if frame changed.
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.playing {
            return false;
        }

        let Some(def) = self.animations.get(&self.current) else {
            return false;
        };

        if def.frames.is_empty() {
            return false;
        }

        self.frame_timer += dt * self.speed;
        let mut frame_changed = false;

        while self.frame_timer >= def.frame_duration {
            self.frame_timer -= def.frame_duration;
            self.frame_index += 1;
            frame_changed = true;

            if self.frame_index >= def.frames.len() {
                if def.looping {
                    self.frame_index = 0;
                } else {
                    self.frame_index = def.frames.len() - 1;
                    self.playing = false;
                    break;
                }
            }
        }

        frame_changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_strip_animation() {
        let def = AnimationDef::horizontal_strip(0.0, 0.0, 4, 10.0);
        assert_eq!(def.frames.len(), 4);
        assert_eq!(def.frames[0], (0.0, 0.0));
        assert_eq!(def.frames[3], (3.0, 0.0));
        assert_eq!(def.frame_duration, 0.1);
    }

    #[test]
    fn animation_ticks_through_frames() {
        let def = AnimationDef::horizontal_strip(0.0, 0.0, 4, 10.0);
        let mut anim = AnimationComponent::single("walk", def);

        assert_eq!(anim.current_frame(), Some((0.0, 0.0)));

        // Tick past first frame
        anim.tick(0.15);
        assert_eq!(anim.frame_index, 1);
        assert_eq!(anim.current_frame(), Some((1.0, 0.0)));

        // Tick through remaining frames
        anim.tick(0.3);
        assert_eq!(anim.frame_index, 0); // Looped back
    }

    #[test]
    fn non_looping_animation_stops() {
        let mut def = AnimationDef::horizontal_strip(0.0, 0.0, 3, 10.0);
        def.looping = false;
        let mut anim = AnimationComponent::single("attack", def);

        anim.tick(0.35); // Should reach end
        assert!(anim.is_finished());
        assert!(!anim.playing);
        assert_eq!(anim.frame_index, 2); // Stuck on last frame
    }

    #[test]
    fn play_if_different() {
        let mut anim = AnimationComponent::default();
        anim.add("idle", AnimationDef::horizontal_strip(0.0, 0.0, 2, 5.0));
        anim.add("walk", AnimationDef::horizontal_strip(1.0, 0.0, 4, 10.0));

        anim.play("idle");
        anim.tick(0.1);
        let old_frame = anim.frame_index;

        // Same animation - should NOT reset
        anim.play_if_different("idle");
        assert_eq!(anim.frame_index, old_frame);

        // Different animation - should reset
        anim.play_if_different("walk");
        assert_eq!(anim.current, "walk");
        assert_eq!(anim.frame_index, 0);
    }
}
