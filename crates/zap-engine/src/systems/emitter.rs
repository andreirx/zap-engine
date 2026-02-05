use crate::core::scene::Scene;
use crate::systems::effects::EffectsState;

/// Tick all emitters attached to active entities, spawning particles into the effects state.
/// This is a free function to avoid borrow conflicts between scene and effects.
pub fn tick_emitters(scene: &mut Scene, effects: &mut EffectsState, dt: f32) {
    for entity in scene.iter_mut() {
        if !entity.active {
            continue;
        }
        let emitter = match &mut entity.emitter {
            Some(e) if e.active => e,
            _ => continue,
        };
        let count = emitter.tick(dt);
        if count == 0 {
            continue;
        }
        let pos = [entity.pos.x, entity.pos.y];
        effects.spawn_particles_with_config(
            pos,
            count,
            emitter.speed_range,
            emitter.width,
            emitter.lifetime,
            &emitter.color_mode,
            emitter.drag,
            emitter.attract_strength,
            emitter.speed_factor,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::EntityId;
    use crate::components::entity::Entity;
    use crate::components::emitter::{EmitterComponent, EmissionMode};
    use glam::Vec2;

    #[test]
    fn tick_emitters_spawns_at_entity_pos() {
        let mut scene = Scene::new();
        let emitter = EmitterComponent::new()
            .with_mode(EmissionMode::Burst)
            .with_burst_count(5)
            .with_burst_interval(0.0);
        scene.spawn(
            Entity::new(EntityId(1))
                .with_pos(Vec2::new(100.0, 200.0))
                .with_emitter(emitter),
        );

        let mut effects = EffectsState::new(42);
        tick_emitters(&mut scene, &mut effects, 0.016);

        assert_eq!(effects.particles.len(), 5);
        for p in &effects.particles {
            assert_eq!(p.position, [100.0, 200.0]);
        }
    }

    #[test]
    fn tick_emitters_skips_inactive_entity() {
        let mut scene = Scene::new();
        let emitter = EmitterComponent::new()
            .with_mode(EmissionMode::Burst)
            .with_burst_count(5)
            .with_burst_interval(0.0);
        let mut entity = Entity::new(EntityId(1))
            .with_pos(Vec2::new(100.0, 200.0))
            .with_emitter(emitter);
        entity.active = false;
        scene.spawn(entity);

        let mut effects = EffectsState::new(42);
        tick_emitters(&mut scene, &mut effects, 0.016);

        assert_eq!(effects.particles.len(), 0);
    }
}
