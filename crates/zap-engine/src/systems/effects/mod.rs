//! Visual effects system: electric arcs, particles, and debug lines.
//!
//! This module provides the `EffectsState` facade for managing all visual effects,
//! plus individual components that can be used directly if needed.

mod rng;
mod segment_color;
mod geometry;
mod electric_arc;
mod particle;
mod debug_line;

// Re-export public types
pub use rng::Rng;
pub use segment_color::{SegmentColor, SegmentUVs};
pub use geometry::{build_strip_vertices, strip_to_triangles};
pub use electric_arc::ElectricArc;
pub use particle::Particle;
pub use debug_line::DebugLine;

/// Container for all visual effects (arcs + particles + debug lines).
/// Generic — games add arcs and particles via public methods.
pub struct EffectsState {
    pub arcs: Vec<(ElectricArc, f32, SegmentColor)>,
    pub particles: Vec<Particle>,
    pub debug_lines: Vec<DebugLine>,
    pub effects_buffer: Vec<f32>,
    pub rng: Rng,
    pub attractor: [f32; 2],
}

impl EffectsState {
    /// Create a new EffectsState with the given RNG seed.
    pub fn new(seed: u64) -> Self {
        EffectsState {
            arcs: Vec::new(),
            particles: Vec::new(),
            debug_lines: Vec::new(),
            effects_buffer: Vec::with_capacity(4096),
            rng: Rng::new(seed.wrapping_add(7919)),
            attractor: [0.0, 0.0],
        }
    }

    /// Create a new EffectsState with a pre-allocated buffer capacity.
    pub fn with_capacity(seed: u64, max_vertices: usize) -> Self {
        EffectsState {
            arcs: Vec::new(),
            particles: Vec::new(),
            debug_lines: Vec::new(),
            effects_buffer: Vec::with_capacity(max_vertices * 5), // 5 floats per vertex
            rng: Rng::new(seed.wrapping_add(7919)),
            attractor: [0.0, 0.0],
        }
    }

    /// Add an electric arc between two points.
    pub fn add_arc(&mut self, start: [f32; 2], end: [f32; 2], width: f32, color: SegmentColor, power_of_two: u32) {
        let arc = ElectricArc::new(start, end, power_of_two, &mut self.rng);
        self.arcs.push((arc, width, color));
    }

    /// Spawn particles at a position with random velocities.
    pub fn spawn_particles(
        &mut self,
        center: [f32; 2],
        count: usize,
        speed_limit: f32,
        width: f32,
        lifetime: f32,
    ) {
        for _ in 0..count {
            let sx = (self.rng.next_int(20000) as f32 / 1000.0) - 10.0;
            let sy = (self.rng.next_int(20000) as f32 / 1000.0) - 10.0;
            let color = SegmentColor::random(&mut self.rng);
            self.particles.push(Particle::new(
                center,
                [sx * speed_limit / 10.0, sy * speed_limit / 10.0],
                width,
                color,
                lifetime,
            ));
        }
    }

    /// Spawn particles with custom physics parameters (used by emitters).
    pub fn spawn_particles_with_config(
        &mut self,
        center: [f32; 2],
        count: usize,
        speed_range: (f32, f32),
        width: f32,
        lifetime: f32,
        color_mode: &crate::components::emitter::ParticleColorMode,
        drag: f32,
        attract_strength: f32,
        speed_factor: f32,
    ) {
        use crate::components::emitter::ParticleColorMode;
        for _ in 0..count {
            let angle = (self.rng.next_int(10000) as f32 / 10000.0) * std::f32::consts::TAU;
            let t = self.rng.next_int(10000) as f32 / 10000.0;
            let speed_mag = speed_range.0 + t * (speed_range.1 - speed_range.0);
            let sx = angle.cos() * speed_mag;
            let sy = angle.sin() * speed_mag;
            let color = match color_mode {
                ParticleColorMode::Random => SegmentColor::random(&mut self.rng),
                ParticleColorMode::Fixed(c) => *c,
                ParticleColorMode::Palette(colors) => {
                    let idx = self.rng.next_int(colors.len() as u32) as usize;
                    colors[idx]
                }
            };
            self.particles.push(Particle {
                position: center,
                speed: [sx, sy],
                width,
                color,
                lifetime,
                drag,
                attract_strength,
                speed_factor,
            });
        }
    }

    /// Advance effects: twitch arcs, update particles.
    pub fn tick(&mut self, dt: f32) {
        for (arc, _, _) in &mut self.arcs {
            arc.twitch(0.05, &mut self.rng);
        }
        let attractor = self.attractor;
        self.particles.retain_mut(|p| p.tick(attractor, dt));
    }

    /// Add a debug line (for collider visualization, paths, etc.).
    pub fn add_debug_line(&mut self, points: Vec<[f32; 2]>, width: f32, color: SegmentColor) {
        self.debug_lines.push(DebugLine::new(points, width, color));
    }

    /// Clear debug lines (call at the start of each frame before re-drawing).
    pub fn clear_debug(&mut self) {
        self.debug_lines.clear();
    }

    /// Rebuild the effects vertex buffer (triangle list, 5 floats per vertex).
    pub fn rebuild_effects_buffer(&mut self) {
        self.effects_buffer.clear();

        for (arc, width, color) in &self.arcs {
            let strip = build_strip_vertices(&arc.points, *width, *color);
            let tris = strip_to_triangles(&strip, 5);
            self.effects_buffer.extend_from_slice(&tris);
        }

        for p in &self.particles {
            let strip = p.to_vertices();
            let tris = strip_to_triangles(&strip, 5);
            self.effects_buffer.extend_from_slice(&tris);
        }

        for line in &self.debug_lines {
            let strip = build_strip_vertices(&line.points, line.width, line.color);
            let tris = strip_to_triangles(&strip, 5);
            self.effects_buffer.extend_from_slice(&tris);
        }
    }

    /// Clear all effects.
    pub fn clear(&mut self) {
        self.arcs.clear();
        self.particles.clear();
        self.debug_lines.clear();
        self.effects_buffer.clear();
    }

    pub fn effects_vertex_count(&self) -> usize {
        self.effects_buffer.len() / 5
    }

    pub fn effects_buffer_ptr(&self) -> *const f32 {
        self.effects_buffer.as_ptr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_state_add_arc() {
        let mut effects = EffectsState::new(42);
        effects.add_arc([0.0, 0.0], [100.0, 0.0], 4.0, SegmentColor::SkyBlue, 3);
        assert_eq!(effects.arcs.len(), 1);
        effects.rebuild_effects_buffer();
        assert!(effects.effects_vertex_count() > 0);
    }

    #[test]
    fn effects_state_spawn_particles() {
        let mut effects = EffectsState::new(42);
        effects.spawn_particles([50.0, 50.0], 10, 10.0, 4.0, 2.0);
        assert_eq!(effects.particles.len(), 10);
    }

    #[test]
    fn effects_state_with_capacity() {
        let effects = EffectsState::with_capacity(42, 1000);
        assert!(effects.effects_buffer.capacity() >= 5000); // 1000 verts * 5 floats
    }

    #[test]
    fn effects_state_clear() {
        let mut effects = EffectsState::new(42);
        effects.add_arc([0.0, 0.0], [100.0, 0.0], 4.0, SegmentColor::Red, 3);
        effects.spawn_particles([50.0, 50.0], 5, 10.0, 4.0, 2.0);
        effects.add_debug_line(vec![[0.0, 0.0], [100.0, 100.0]], 2.0, SegmentColor::White);

        effects.clear();

        assert!(effects.arcs.is_empty());
        assert!(effects.particles.is_empty());
        assert!(effects.debug_lines.is_empty());
    }
}
