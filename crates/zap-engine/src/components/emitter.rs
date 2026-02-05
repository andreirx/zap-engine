use crate::systems::effects::SegmentColor;

/// How the emitter releases particles.
#[derive(Debug, Clone)]
pub enum EmissionMode {
    /// Emit particles continuously at a fixed rate.
    Continuous,
    /// Emit particles in bursts.
    Burst,
}

/// How particle colors are chosen.
#[derive(Debug, Clone)]
pub enum ParticleColorMode {
    /// Pick a random SegmentColor for each particle.
    Random,
    /// All particles use the same color.
    Fixed(SegmentColor),
    /// Pick randomly from a palette of colors.
    Palette(Vec<SegmentColor>),
}

/// Component for auto-spawning particles from an entity's position.
#[derive(Debug, Clone)]
pub struct EmitterComponent {
    /// Whether the emitter is actively spawning.
    pub active: bool,
    /// Emission mode (continuous or burst).
    pub mode: EmissionMode,
    /// Particles per second (Continuous mode).
    pub rate: f32,
    /// Particles per burst (Burst mode).
    pub burst_count: u32,
    /// Seconds between bursts (0 = one-shot).
    pub burst_interval: f32,
    /// Min/max initial speed magnitude.
    pub speed_range: (f32, f32),
    /// Particle visual width.
    pub width: f32,
    /// Particle lifetime in seconds.
    pub lifetime: f32,
    /// How particle colors are chosen.
    pub color_mode: ParticleColorMode,
    /// Per-particle drag coefficient.
    pub drag: f32,
    /// Per-particle attractor strength.
    pub attract_strength: f32,
    /// Per-particle speed factor.
    pub speed_factor: f32,
    /// Internal accumulator for continuous emission.
    accumulator: f32,
    /// Internal timer for burst intervals.
    burst_timer: f32,
    /// Whether the first burst has fired (for one-shot bursts).
    burst_fired: bool,
}

impl Default for EmitterComponent {
    fn default() -> Self {
        Self {
            active: true,
            mode: EmissionMode::Continuous,
            rate: 10.0,
            burst_count: 8,
            burst_interval: 0.0,
            speed_range: (2.0, 8.0),
            width: 4.0,
            lifetime: 1.0,
            color_mode: ParticleColorMode::Random,
            drag: 0.02,
            attract_strength: 0.3,
            speed_factor: 0.8,
            accumulator: 0.0,
            burst_timer: 0.0,
            burst_fired: false,
        }
    }
}

impl EmitterComponent {
    pub fn new() -> Self {
        Self::default()
    }

    // -- Builder pattern --

    pub fn with_mode(mut self, mode: EmissionMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_rate(mut self, rate: f32) -> Self {
        self.rate = rate;
        self
    }

    pub fn with_burst_count(mut self, count: u32) -> Self {
        self.burst_count = count;
        self
    }

    pub fn with_burst_interval(mut self, interval: f32) -> Self {
        self.burst_interval = interval;
        self
    }

    pub fn with_speed_range(mut self, min: f32, max: f32) -> Self {
        self.speed_range = (min, max);
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_lifetime(mut self, lifetime: f32) -> Self {
        self.lifetime = lifetime;
        self
    }

    pub fn with_color_mode(mut self, mode: ParticleColorMode) -> Self {
        self.color_mode = mode;
        self
    }

    pub fn with_drag(mut self, drag: f32) -> Self {
        self.drag = drag;
        self
    }

    pub fn with_attract_strength(mut self, strength: f32) -> Self {
        self.attract_strength = strength;
        self
    }

    pub fn with_speed_factor(mut self, factor: f32) -> Self {
        self.speed_factor = factor;
        self
    }

    /// Advance the emitter by `dt` seconds. Returns the number of particles to spawn.
    pub fn tick(&mut self, dt: f32) -> usize {
        if !self.active {
            return 0;
        }

        match &self.mode {
            EmissionMode::Continuous => {
                self.accumulator += self.rate * dt;
                let count = self.accumulator as usize;
                self.accumulator -= count as f32;
                count
            }
            EmissionMode::Burst => {
                if self.burst_interval <= 0.0 {
                    // One-shot burst
                    if !self.burst_fired {
                        self.burst_fired = true;
                        self.burst_count as usize
                    } else {
                        0
                    }
                } else {
                    self.burst_timer += dt;
                    if self.burst_timer >= self.burst_interval {
                        self.burst_timer -= self.burst_interval;
                        self.burst_count as usize
                    } else {
                        0
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_emitter() {
        let e = EmitterComponent::default();
        assert!(e.active);
        assert_eq!(e.rate, 10.0);
        assert_eq!(e.drag, 0.02);
    }

    #[test]
    fn builder_pattern() {
        let e = EmitterComponent::new()
            .with_rate(50.0)
            .with_mode(EmissionMode::Burst)
            .with_burst_count(16)
            .with_burst_interval(0.5)
            .with_drag(0.05);
        assert_eq!(e.rate, 50.0);
        assert_eq!(e.burst_count, 16);
        assert_eq!(e.burst_interval, 0.5);
        assert_eq!(e.drag, 0.05);
    }

    #[test]
    fn continuous_accumulator() {
        let mut e = EmitterComponent::new().with_rate(60.0);
        // At 60 particles/sec, 1/60 sec should yield ~1 particle
        let count = e.tick(1.0 / 60.0);
        assert_eq!(count, 1);
    }

    #[test]
    fn burst_one_shot() {
        let mut e = EmitterComponent::new()
            .with_mode(EmissionMode::Burst)
            .with_burst_count(10)
            .with_burst_interval(0.0);
        assert_eq!(e.tick(0.016), 10);
        // Second tick should yield 0 (one-shot)
        assert_eq!(e.tick(0.016), 0);
    }

    #[test]
    fn burst_repeating() {
        let mut e = EmitterComponent::new()
            .with_mode(EmissionMode::Burst)
            .with_burst_count(5)
            .with_burst_interval(1.0);
        // Not enough time yet
        assert_eq!(e.tick(0.5), 0);
        // Now enough time
        assert_eq!(e.tick(0.6), 5);
    }
}
