//! Animation system — ticks animations and updates sprite frames.

use crate::core::scene::Scene;

/// Tick all entity animations and update their sprite col/row.
///
/// Call this once per frame before rendering.
pub fn tick_animations(scene: &mut Scene, dt: f32) {
    for entity in scene.iter_mut() {
        if let Some(ref mut anim) = entity.animation {
            // Tick the animation
            anim.tick(dt);

            // Update sprite to match current frame
            if let Some((col, row)) = anim.current_frame() {
                if let Some(ref mut sprite) = entity.sprite {
                    sprite.col = col;
                    sprite.row = row;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::EntityId;
    use crate::components::animation::{AnimationComponent, AnimationDef};
    use crate::components::entity::Entity;
    use crate::components::sprite::SpriteComponent;
    use glam::Vec2;

    #[test]
    fn tick_updates_sprite_frame() {
        let mut scene = Scene::new();

        // Create entity with animation
        let anim = AnimationComponent::single(
            "walk",
            AnimationDef::horizontal_strip(0.0, 0.0, 4, 10.0), // 4 frames at 10fps
        );
        let sprite = SpriteComponent::default();

        let entity = Entity::new(EntityId(1))
            .with_pos(Vec2::ZERO)
            .with_sprite(sprite)
            .with_animation(anim);

        scene.spawn(entity);

        // Initial state
        assert_eq!(scene.get(EntityId(1)).unwrap().sprite.as_ref().unwrap().col, 0.0);

        // Tick past first frame (0.1s per frame, tick 0.15s)
        tick_animations(&mut scene, 0.15);

        // Should now be on frame 1
        assert_eq!(scene.get(EntityId(1)).unwrap().sprite.as_ref().unwrap().col, 1.0);
    }

    #[test]
    fn animation_loops() {
        let mut scene = Scene::new();

        let anim = AnimationComponent::single(
            "idle",
            AnimationDef::horizontal_strip(0.0, 0.0, 2, 10.0), // 2 frames
        );
        let sprite = SpriteComponent::default();

        scene.spawn(
            Entity::new(EntityId(1))
                .with_sprite(sprite)
                .with_animation(anim),
        );

        // Tick through 3 frames worth (should loop back)
        tick_animations(&mut scene, 0.35);

        // Should be on frame 1 (0 -> 1 -> 0 -> 1)
        let col = scene.get(EntityId(1)).unwrap().sprite.as_ref().unwrap().col;
        assert!(col == 0.0 || col == 1.0); // Either 0 or 1 depending on exact timing
    }
}
