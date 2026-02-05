/// Fixed timestep accumulator.
/// Ensures game logic runs at a consistent rate regardless of frame time.
pub struct FixedTimestep {
    /// The fixed delta time per tick.
    dt: f32,
    /// Accumulated time from variable frame deltas.
    accumulator: f32,
}

impl FixedTimestep {
    pub fn new(dt: f32) -> Self {
        Self {
            dt,
            accumulator: 0.0,
        }
    }

    /// Add frame time to the accumulator. Returns the number of fixed steps to run.
    pub fn accumulate(&mut self, frame_dt: f32) -> u32 {
        self.accumulator += frame_dt;
        // Cap to prevent spiral of death (max 10 steps per frame)
        self.accumulator = self.accumulator.min(self.dt * 10.0);
        let steps = (self.accumulator / self.dt) as u32;
        self.accumulator -= steps as f32 * self.dt;
        steps
    }

    /// Interpolation alpha for rendering between ticks (0.0 to 1.0).
    pub fn alpha(&self) -> f32 {
        self.accumulator / self.dt
    }

    /// The fixed delta time.
    pub fn dt(&self) -> f32 {
        self.dt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_step_exact() {
        let mut ts = FixedTimestep::new(1.0 / 60.0);
        let steps = ts.accumulate(1.0 / 60.0);
        assert_eq!(steps, 1);
    }

    #[test]
    fn accumulates_partial() {
        let mut ts = FixedTimestep::new(1.0 / 60.0);
        let steps = ts.accumulate(0.008); // half a frame
        assert_eq!(steps, 0);
        let steps = ts.accumulate(0.010); // over one frame total
        assert_eq!(steps, 1);
    }

    #[test]
    fn caps_at_ten_steps() {
        let mut ts = FixedTimestep::new(1.0 / 60.0);
        let steps = ts.accumulate(1.0); // 60 frames worth, but capped at 10
        assert_eq!(steps, 10);
    }

    #[test]
    fn alpha_is_between_zero_and_one() {
        let mut ts = FixedTimestep::new(1.0 / 60.0);
        ts.accumulate(0.008);
        let a = ts.alpha();
        assert!(a >= 0.0 && a <= 1.0, "alpha was {}", a);
    }
}
