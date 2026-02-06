use std::f32::consts::FRAC_PI_2;

/// Rotation animation for a tile being tapped.
/// Animates from start_rotation to 0 over 0.2 seconds.
#[derive(Debug, Clone)]
pub struct RotateAnim {
    pub x: usize,
    pub y: usize,
    pub start_rotation: f32,
    pub progress: f32,
    pub duration: f32,
}

impl RotateAnim {
    pub fn new(x: usize, y: usize, rotation_count: usize) -> Self {
        let count = rotation_count.max(1) as f32;
        RotateAnim {
            x,
            y,
            start_rotation: count * FRAC_PI_2,
            progress: 0.0,
            duration: 0.2,
        }
    }

    /// Advance animation. Returns current rotation angle, or None when complete.
    pub fn tick(&mut self, dt: f32) -> Option<f32> {
        self.progress += dt / self.duration;
        if self.progress >= 1.0 {
            return None;
        }
        // Lerp from start_rotation to 0
        Some(self.start_rotation * (1.0 - self.progress))
    }
}

/// Gravity-based fall animation for tiles after a zap.
#[derive(Debug, Clone)]
pub struct FallAnim {
    pub x: usize,
    pub y: usize,
    pub current_y: f32,
    pub target_y: f32,
    pub speed: f32,
}

impl FallAnim {
    const GRAVITY: f32 = 9.8;
    const FRICTION: f32 = 0.005;
    const SPEED_FACTOR: f32 = 1.0;
    const DT: f32 = 1.0 / 60.0;

    pub fn new(x: usize, y: usize, start_y: f32, target_y: f32) -> Self {
        FallAnim {
            x,
            y,
            current_y: start_y,
            target_y,
            speed: 0.0,
        }
    }

    /// Advance physics one frame. Returns current Y, or None when settled.
    pub fn tick(&mut self) -> Option<f32> {
        self.speed += Self::GRAVITY * Self::SPEED_FACTOR * Self::DT;
        self.speed *= 1.0 - Self::FRICTION;
        self.current_y += self.speed;

        if self.current_y >= self.target_y {
            return None;
        }
        Some(self.current_y)
    }
}

/// Container for all active animations.
#[derive(Debug, Default)]
pub struct AnimationState {
    pub rotate_anims: Vec<RotateAnim>,
    pub fall_anims: Vec<FallAnim>,
    pub freeze_timer: f32,
}

impl AnimationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_rotate_anims(&self) -> bool {
        !self.rotate_anims.is_empty()
    }

    pub fn has_fall_anims(&self) -> bool {
        !self.fall_anims.is_empty()
    }

    pub fn is_frozen(&self) -> bool {
        self.freeze_timer > 0.0
    }

    pub fn tick_rotations(&mut self, dt: f32) {
        self.rotate_anims.retain_mut(|anim| anim.tick(dt).is_some());
    }

    pub fn tick_falls(&mut self) {
        self.fall_anims.retain_mut(|anim| anim.tick().is_some());
    }

    /// Returns true if freeze just ended.
    pub fn tick_freeze(&mut self, dt: f32) -> bool {
        if self.freeze_timer > 0.0 {
            self.freeze_timer -= dt;
            if self.freeze_timer <= 0.0 {
                self.freeze_timer = 0.0;
                return true;
            }
        }
        false
    }

    pub fn get_rotation(&self, x: usize, y: usize) -> Option<f32> {
        for anim in &self.rotate_anims {
            if anim.x == x && anim.y == y {
                return Some(anim.start_rotation * (1.0 - anim.progress));
            }
        }
        None
    }

    pub fn get_fall_y(&self, x: usize, y: usize) -> Option<f32> {
        for anim in &self.fall_anims {
            if anim.x == x && anim.y == y {
                return Some(anim.current_y);
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.rotate_anims.clear();
        self.fall_anims.clear();
        self.freeze_timer = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_anim_completes() {
        let mut anim = RotateAnim::new(5, 5, 1);
        assert!(anim.tick(0.1).is_some());
        assert!(anim.tick(0.11).is_none());
    }

    #[test]
    fn rotate_anim_interpolates() {
        let mut anim = RotateAnim::new(0, 0, 1);
        let rot = anim.tick(0.1).unwrap(); // 50%
        let expected = FRAC_PI_2 * 0.5;
        assert!((rot - expected).abs() < 0.01);
    }

    #[test]
    fn fall_anim_reaches_target() {
        let mut anim = FallAnim::new(0, 0, 0.0, 100.0);
        let mut frames = 0;
        loop {
            if anim.tick().is_none() {
                break;
            }
            frames += 1;
            assert!(frames < 600, "fall anim didn't settle");
        }
        assert!(frames > 0);
    }

    #[test]
    fn freeze_timer_expires() {
        let mut state = AnimationState::new();
        state.freeze_timer = 1.0;
        assert!(state.is_frozen());
        assert!(!state.tick_freeze(0.5));
        assert!(state.tick_freeze(0.6));
        assert!(!state.is_frozen());
    }
}
