use crate::components::entity::Entity;
use crate::renderer::instance::{RenderBuffer, RenderInstance};

/// Build the render buffer from a set of entities.
/// Groups entities by atlas: atlas 0 first, then other atlases (1+).
/// Sets `atlas_split` at the boundary between atlas 0 and atlas 1+.
///
/// This enables multi-atlas rendering (e.g., game sprites on atlas 0, fonts on atlas 1).
pub fn build_render_buffer<'a>(entities: impl Iterator<Item = &'a Entity>, buffer: &mut RenderBuffer) {
    buffer.clear();

    let mut atlas0_instances: Vec<RenderInstance> = Vec::new();
    let mut other_instances: Vec<RenderInstance> = Vec::new();

    for entity in entities {
        if !entity.active {
            continue;
        }

        let sprite = match &entity.sprite {
            Some(s) => s,
            None => continue,
        };

        let instance = RenderInstance {
            x: entity.pos.x,
            y: entity.pos.y,
            rotation: entity.rotation,
            scale: entity.scale.x,
            sprite_col: sprite.col,
            alpha: sprite.alpha,
            cell_span: sprite.cell_span,
            atlas_row: sprite.row,
        };

        // Split by atlas index: atlas 0 in first bucket, all others in second bucket
        match sprite.atlas.0 {
            0 => atlas0_instances.push(instance),
            _ => other_instances.push(instance),
        }
    }

    let split = atlas0_instances.len() as u32;

    for inst in atlas0_instances {
        buffer.push(inst);
    }
    buffer.set_atlas_split(split);
    for inst in other_instances {
        buffer.push(inst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::EntityId;
    use crate::components::sprite::{AtlasId, SpriteComponent};
    use glam::Vec2;

    #[test]
    fn build_buffer_splits_by_atlas_id() {
        let entities = vec![
            // Atlas 0 sprite
            Entity::new(EntityId(1))
                .with_pos(Vec2::new(10.0, 20.0))
                .with_scale(Vec2::splat(50.0))
                .with_sprite(SpriteComponent {
                    atlas: AtlasId(0),
                    ..Default::default()
                }),
            // Atlas 1 sprite (e.g., font)
            Entity::new(EntityId(2))
                .with_pos(Vec2::new(30.0, 40.0))
                .with_scale(Vec2::splat(50.0))
                .with_sprite(SpriteComponent {
                    atlas: AtlasId(1),
                    ..Default::default()
                }),
            // Another atlas 0 sprite
            Entity::new(EntityId(3))
                .with_pos(Vec2::new(50.0, 60.0))
                .with_scale(Vec2::splat(50.0))
                .with_sprite(SpriteComponent {
                    atlas: AtlasId(0),
                    ..Default::default()
                }),
            // Another atlas 1 sprite
            Entity::new(EntityId(4))
                .with_pos(Vec2::new(70.0, 80.0))
                .with_scale(Vec2::splat(50.0))
                .with_sprite(SpriteComponent {
                    atlas: AtlasId(1),
                    ..Default::default()
                }),
        ];

        let mut buffer = RenderBuffer::new();
        build_render_buffer(entities.iter(), &mut buffer);

        assert_eq!(buffer.instance_count(), 4);
        // Atlas 0 sprites (entities 1 and 3) go first, atlas 1 sprites (2 and 4) go after
        assert_eq!(buffer.atlas_split, 2); // 2 atlas-0, 2 atlas-1
    }

    #[test]
    fn inactive_entities_are_skipped() {
        let mut entity = Entity::new(EntityId(1))
            .with_sprite(SpriteComponent::default());
        entity.active = false;

        let entities = vec![entity];
        let mut buffer = RenderBuffer::new();
        build_render_buffer(entities.iter(), &mut buffer);
        assert_eq!(buffer.instance_count(), 0);
    }
}
