use crate::components::entity::Entity;
use crate::components::sprite::BlendMode;
use crate::renderer::instance::{RenderBuffer, RenderInstance};

/// Build the render buffer from a set of entities.
/// Groups entities by blend mode: alpha-blended first (atlas 0), then additive.
/// Sets `atlas_split` at the boundary.
pub fn build_render_buffer<'a>(entities: impl Iterator<Item = &'a Entity>, buffer: &mut RenderBuffer) {
    buffer.clear();

    let mut alpha_instances: Vec<RenderInstance> = Vec::new();
    let mut additive_instances: Vec<RenderInstance> = Vec::new();

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

        match sprite.blend {
            BlendMode::Alpha => alpha_instances.push(instance),
            BlendMode::Additive => additive_instances.push(instance),
        }
    }

    let split = alpha_instances.len() as u32;

    for inst in alpha_instances {
        buffer.push(inst);
    }
    buffer.set_atlas_split(split);
    for inst in additive_instances {
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
    fn build_buffer_groups_by_blend_mode() {
        let entities = vec![
            Entity::new(EntityId(1))
                .with_pos(Vec2::new(10.0, 20.0))
                .with_scale(Vec2::splat(50.0))
                .with_sprite(SpriteComponent {
                    blend: BlendMode::Alpha,
                    ..Default::default()
                }),
            Entity::new(EntityId(2))
                .with_pos(Vec2::new(30.0, 40.0))
                .with_scale(Vec2::splat(50.0))
                .with_sprite(SpriteComponent {
                    blend: BlendMode::Additive,
                    ..Default::default()
                }),
            Entity::new(EntityId(3))
                .with_pos(Vec2::new(50.0, 60.0))
                .with_scale(Vec2::splat(50.0))
                .with_sprite(SpriteComponent {
                    atlas: AtlasId(0),
                    blend: BlendMode::Alpha,
                    ..Default::default()
                }),
        ];

        let mut buffer = RenderBuffer::new();
        build_render_buffer(entities.iter(), &mut buffer);

        assert_eq!(buffer.instance_count(), 3);
        assert_eq!(buffer.atlas_split, 2); // 2 alpha, 1 additive
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
