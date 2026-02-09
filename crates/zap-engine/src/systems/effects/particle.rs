//! Particle system for visual effects.

use super::geometry::build_strip_vertices;
use super::segment_color::SegmentColor;

/// A single particle with physics and rendering state.
#[derive(Debug, Clone)]
pub struct Particle {
    pub position: [f32; 2],
    pub speed: [f32; 2],
    pub width: f32,
    pub color: SegmentColor,
    pub lifetime: f32,
    pub drag: f32,
    pub attract_strength: f32,
    pub speed_factor: f32,
}

impl Particle {
    pub const DEFAULT_DRAG: f32 = 0.02;
    pub const DEFAULT_ATTRACT_STRENGTH: f32 = 0.3;
    pub const DEFAULT_SPEED_FACTOR: f32 = 0.8;

    pub fn new(position: [f32; 2], speed: [f32; 2], width: f32, color: SegmentColor, lifetime: f32) -> Self {
        Particle {
            position, speed, width, color, lifetime,
            drag: Self::DEFAULT_DRAG,
            attract_strength: Self::DEFAULT_ATTRACT_STRENGTH,
            speed_factor: Self::DEFAULT_SPEED_FACTOR,
        }
    }

    /// Advance particle physics. Returns false when expired.
    pub fn tick(&mut self, attractor: [f32; 2], dt: f32) -> bool {
        self.lifetime -= dt;
        if self.lifetime <= 0.0 {
            return false;
        }

        let dx = attractor[0] - self.position[0];
        let dy = attractor[1] - self.position[1];
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let to_attr = [dx / len, dy / len];

        self.speed[0] += to_attr[0] * self.attract_strength;
        self.speed[1] += to_attr[1] * self.attract_strength;
        self.speed[0] *= 1.0 - self.drag;
        self.speed[1] *= 1.0 - self.drag;
        self.position[0] += self.speed[0] * self.speed_factor;
        self.position[1] += self.speed[1] * self.speed_factor;

        true
    }

    /// Generate vertices for this particle (2-point segment strip).
    pub fn to_vertices(&self) -> Vec<f32> {
        let end = [
            self.position[0] + self.speed[0],
            self.position[1] + self.speed[1],
        ];
        build_strip_vertices(
            &[self.position, end],
            self.width,
            self.color,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_expires() {
        let mut p = Particle::new([0.0, 0.0], [1.0, 0.0], 4.0, SegmentColor::Red, 0.1);
        let alive = p.tick([0.0, 100.0], 0.2);
        assert!(!alive, "particle should expire");
    }

    #[test]
    fn particle_lives_while_lifetime_positive() {
        let mut p = Particle::new([0.0, 0.0], [1.0, 0.0], 4.0, SegmentColor::Red, 1.0);
        let alive = p.tick([0.0, 100.0], 0.1);
        assert!(alive, "particle should still be alive");
    }

    #[test]
    fn particle_moves_toward_attractor() {
        let mut p = Particle::new([0.0, 0.0], [0.0, 0.0], 4.0, SegmentColor::Red, 10.0);
        let attractor = [100.0, 0.0];
        p.tick(attractor, 0.1);
        assert!(p.position[0] > 0.0, "particle should move toward attractor");
    }

    #[test]
    fn particle_to_vertices_produces_output() {
        let p = Particle::new([0.0, 0.0], [10.0, 0.0], 4.0, SegmentColor::Blue, 1.0);
        let verts = p.to_vertices();
        assert!(!verts.is_empty());
    }
}
