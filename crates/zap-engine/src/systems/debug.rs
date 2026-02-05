//! Debug rendering — opt-in collider visualization via the effects pipeline.
//!
//! Call `debug_draw_colliders()` from your `Game::update()` to see hitboxes.
//! Debug lines are rendered as additive glow strips (same pipeline as arcs).

#[cfg(feature = "physics")]
use crate::core::physics::{ColliderDesc, PhysicsWorld};
#[cfg(feature = "physics")]
use crate::core::scene::Scene;
#[cfg(feature = "physics")]
use crate::systems::effects::{EffectsState, SegmentColor};

/// Draw wireframe outlines for all physics colliders in the scene.
///
/// Clears previous debug lines and rebuilds them from scratch.
/// Call this each frame from `Game::update()` to keep debug visuals in sync.
///
/// # Arguments
/// - `scene` — the entity scene to iterate
/// - `physics` — the physics world to query collider shapes from
/// - `effects` — the effects state to add debug lines into
/// - `line_width` — visual width of the debug lines in world units
/// - `color` — the segment color for the debug lines
#[cfg(feature = "physics")]
pub fn debug_draw_colliders(
    scene: &Scene,
    physics: &PhysicsWorld,
    effects: &mut EffectsState,
    line_width: f32,
    color: SegmentColor,
) {
    effects.clear_debug();
    for entity in scene.iter() {
        if !entity.active {
            continue;
        }
        let body = match &entity.body {
            Some(b) => b,
            None => continue,
        };
        let (pos, rot) = physics.body_position(body);
        let shape = match physics.collider_shape(body) {
            Some(s) => s,
            None => continue,
        };
        let points = collider_outline(pos.x, pos.y, rot, &shape);
        effects.add_debug_line(points, line_width, color);
    }
}

/// Generate outline points for a collider shape at a given position and rotation.
#[cfg(feature = "physics")]
fn collider_outline(cx: f32, cy: f32, rot: f32, shape: &ColliderDesc) -> Vec<[f32; 2]> {
    match *shape {
        ColliderDesc::Ball { radius } => {
            // 24-segment circle
            let segments = 24;
            let mut points = Vec::with_capacity(segments + 1);
            for i in 0..=segments {
                let angle = rot + (i as f32 / segments as f32) * std::f32::consts::TAU;
                points.push([
                    cx + angle.cos() * radius,
                    cy + angle.sin() * radius,
                ]);
            }
            points
        }
        ColliderDesc::Cuboid {
            half_width,
            half_height,
        } => {
            // Rotated rectangle (4 corners + close)
            let cos_r = rot.cos();
            let sin_r = rot.sin();
            let corners: [[f32; 2]; 4] = [
                [-half_width, -half_height],
                [half_width, -half_height],
                [half_width, half_height],
                [-half_width, half_height],
            ];
            let mut points = Vec::with_capacity(5);
            for [lx, ly] in &corners {
                points.push([
                    cx + lx * cos_r - ly * sin_r,
                    cy + lx * sin_r + ly * cos_r,
                ]);
            }
            // Close the loop
            points.push(points[0]);
            points
        }
        ColliderDesc::CapsuleY {
            half_height,
            radius,
        } => {
            // Capsule: semicircle top, straight sides, semicircle bottom
            let cos_r = rot.cos();
            let sin_r = rot.sin();
            let rotate = |lx: f32, ly: f32| -> [f32; 2] {
                [cx + lx * cos_r - ly * sin_r, cy + lx * sin_r + ly * cos_r]
            };

            let semi_segments = 12;
            let mut points = Vec::with_capacity(semi_segments * 2 + 4);

            // Top semicircle (at y = -half_height)
            for i in 0..=semi_segments {
                let angle = std::f32::consts::PI + (i as f32 / semi_segments as f32) * std::f32::consts::PI;
                let lx = angle.cos() * radius;
                let ly = -half_height + angle.sin() * radius;
                points.push(rotate(lx, ly));
            }

            // Bottom semicircle (at y = +half_height)
            for i in 0..=semi_segments {
                let angle = (i as f32 / semi_segments as f32) * std::f32::consts::PI;
                let lx = angle.cos() * radius;
                let ly = half_height + angle.sin() * radius;
                points.push(rotate(lx, ly));
            }

            // Close the loop
            if let Some(&first) = points.first() {
                points.push(first);
            }
            points
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "physics")]
    use crate::core::physics::{BodyDesc, ColliderMaterial, PhysicsWorld};
    #[cfg(feature = "physics")]
    use crate::api::types::EntityId;
    #[cfg(feature = "physics")]
    use crate::components::entity::Entity;
    #[cfg(feature = "physics")]
    use glam::Vec2;

    #[cfg(feature = "physics")]
    #[test]
    fn debug_draw_populates_debug_lines() {
        let mut scene = Scene::new();
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let mut effects = EffectsState::new(42);

        let id = EntityId(1);
        let mut entity = Entity::new(id).with_pos(Vec2::new(100.0, 200.0));
        let body = physics.create_body(
            id,
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 20.0 }).with_position(Vec2::new(100.0, 200.0)),
            ColliderMaterial::default(),
        );
        entity.body = Some(body);
        scene.spawn(entity);

        assert!(effects.debug_lines.is_empty());
        debug_draw_colliders(&scene, &physics, &mut effects, 2.0, SegmentColor::Green);
        assert_eq!(effects.debug_lines.len(), 1);
        assert!(effects.debug_lines[0].points.len() >= 24);
    }

    #[cfg(feature = "physics")]
    #[test]
    fn debug_lines_included_in_effects_buffer() {
        let mut effects = EffectsState::new(42);
        effects.add_debug_line(
            vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]],
            2.0,
            SegmentColor::White,
        );
        effects.rebuild_effects_buffer();
        assert!(
            effects.effects_buffer.len() > 0,
            "Effects buffer should contain debug line vertices"
        );
    }

    #[test]
    fn clear_debug_empties_lines() {
        let mut effects = EffectsState::new(42);
        effects.add_debug_line(
            vec![[0.0, 0.0], [10.0, 10.0]],
            1.0,
            SegmentColor::Red,
        );
        assert_eq!(effects.debug_lines.len(), 1);
        effects.clear_debug();
        assert!(effects.debug_lines.is_empty());
    }
}
