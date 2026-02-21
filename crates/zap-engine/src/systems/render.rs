use crate::components::entity::Entity;
use crate::components::layer::RenderLayer;
use crate::renderer::instance::{RenderBuffer, RenderInstance};

/// Describes a contiguous batch of instances sharing the same layer AND atlas.
/// One batch per (layer, atlas) pair enables N-atlas rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerBatch {
    /// Which render layer this batch belongs to.
    pub layer: RenderLayer,
    /// Start index (inclusive) in the render buffer.
    pub start: u32,
    /// End index (exclusive) in the render buffer.
    pub end: u32,
    /// Which atlas this batch uses (index into manifest's atlas list).
    pub atlas_id: u32,
}

impl LayerBatch {
    /// Floats per LayerBatch in the protocol wire format.
    pub const FLOATS: usize = 4;
}

/// Build the render buffer from a set of entities.
/// Sorts entities by (layer, atlas) for layered rendering with N-atlas support.
/// Returns one LayerBatch per (layer, atlas) pair.
///
/// Draw order: layers back-to-front, within each layer atlases in ascending order.
pub fn build_render_buffer<'a>(
    entities: impl Iterator<Item = &'a Entity>,
    buffer: &mut RenderBuffer,
) -> Vec<LayerBatch> {
    buffer.clear();

    // Collect active sprite entities with their sort key
    struct SortEntry {
        layer: RenderLayer,
        atlas: u32,
        instance: RenderInstance,
    }

    let mut entries: Vec<SortEntry> = Vec::new();

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

        entries.push(SortEntry {
            layer: entity.layer,
            atlas: sprite.atlas.0,
            instance,
        });
    }

    // Sort by (layer, atlas) — full atlas ID ordering for N-atlas support
    entries.sort_by(|a, b| {
        a.layer.cmp(&b.layer)
            .then_with(|| a.atlas.cmp(&b.atlas))
    });

    // Build buffer and extract batch boundaries — one batch per (layer, atlas) pair
    let mut batches: Vec<LayerBatch> = Vec::new();
    let mut current_key: Option<(RenderLayer, u32)> = None;
    let mut batch_start: u32 = 0;

    for entry in &entries {
        let idx = buffer.instance_count();
        let key = (entry.layer, entry.atlas);

        if current_key != Some(key) {
            // Close previous batch
            if let Some((layer, atlas)) = current_key {
                batches.push(LayerBatch {
                    layer,
                    start: batch_start,
                    end: idx,
                    atlas_id: atlas,
                });
            }
            // Start new batch
            current_key = Some(key);
            batch_start = idx;
        }

        buffer.push(entry.instance);
    }

    // Close final batch
    if let Some((layer, atlas)) = current_key {
        batches.push(LayerBatch {
            layer,
            start: batch_start,
            end: buffer.instance_count(),
            atlas_id: atlas,
        });
    }

    // Set legacy atlas_split for backward compatibility:
    // Count all instances using atlas 0 (for legacy renderers without batch support).
    let total_atlas0: u32 = batches.iter()
        .filter(|b| b.atlas_id == 0)
        .map(|b| b.end - b.start)
        .sum();
    buffer.set_atlas_split(total_atlas0);

    batches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::EntityId;
    use crate::components::sprite::{AtlasId, SpriteComponent};
    use glam::Vec2;

    #[test]
    fn build_buffer_creates_per_atlas_batches() {
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
        let batches = build_render_buffer(entities.iter(), &mut buffer);

        assert_eq!(buffer.instance_count(), 4);
        // All entities are on Objects layer but different atlases → two batches
        assert_eq!(batches.len(), 2);

        // First batch: atlas 0 instances (2 entities)
        assert_eq!(batches[0].layer, RenderLayer::Objects);
        assert_eq!(batches[0].atlas_id, 0);
        assert_eq!(batches[0].start, 0);
        assert_eq!(batches[0].end, 2);

        // Second batch: atlas 1 instances (2 entities)
        assert_eq!(batches[1].layer, RenderLayer::Objects);
        assert_eq!(batches[1].atlas_id, 1);
        assert_eq!(batches[1].start, 2);
        assert_eq!(batches[1].end, 4);

        // Legacy compat: atlas_split = count of atlas-0 instances
        assert_eq!(buffer.atlas_split, 2);
    }

    #[test]
    fn inactive_entities_are_skipped() {
        let mut entity = Entity::new(EntityId(1))
            .with_sprite(SpriteComponent::default());
        entity.active = false;

        let entities = vec![entity];
        let mut buffer = RenderBuffer::new();
        let batches = build_render_buffer(entities.iter(), &mut buffer);
        assert_eq!(buffer.instance_count(), 0);
        assert!(batches.is_empty());
    }

    #[test]
    fn entities_sort_by_layer() {
        let entities = vec![
            Entity::new(EntityId(1))
                .with_layer(RenderLayer::UI)
                .with_pos(Vec2::new(10.0, 10.0))
                .with_sprite(SpriteComponent::default()),
            Entity::new(EntityId(2))
                .with_layer(RenderLayer::Background)
                .with_pos(Vec2::new(20.0, 20.0))
                .with_sprite(SpriteComponent::default()),
            Entity::new(EntityId(3))
                .with_layer(RenderLayer::Objects)
                .with_pos(Vec2::new(30.0, 30.0))
                .with_sprite(SpriteComponent::default()),
        ];

        let mut buffer = RenderBuffer::new();
        let batches = build_render_buffer(entities.iter(), &mut buffer);

        assert_eq!(buffer.instance_count(), 3);
        assert_eq!(batches.len(), 3);

        // Background first
        assert_eq!(batches[0].layer, RenderLayer::Background);
        assert_eq!(batches[0].start, 0);
        assert_eq!(batches[0].end, 1);

        // Objects second
        assert_eq!(batches[1].layer, RenderLayer::Objects);
        assert_eq!(batches[1].start, 1);
        assert_eq!(batches[1].end, 2);

        // UI last
        assert_eq!(batches[2].layer, RenderLayer::UI);
        assert_eq!(batches[2].start, 2);
        assert_eq!(batches[2].end, 3);

        // Verify draw order: Background entity (pos 20,20) is at index 0
        let instances = &buffer.instances;
        assert_eq!(instances[0].x, 20.0); // Background
        assert_eq!(instances[1].x, 30.0); // Objects
        assert_eq!(instances[2].x, 10.0); // UI
    }

    #[test]
    fn atlas_grouping_within_layers() {
        let entities = vec![
            // Background layer, atlas 1
            Entity::new(EntityId(1))
                .with_layer(RenderLayer::Background)
                .with_sprite(SpriteComponent { atlas: AtlasId(1), ..Default::default() }),
            // Background layer, atlas 0
            Entity::new(EntityId(2))
                .with_layer(RenderLayer::Background)
                .with_sprite(SpriteComponent { atlas: AtlasId(0), ..Default::default() }),
            // Objects layer, atlas 0
            Entity::new(EntityId(3))
                .with_layer(RenderLayer::Objects)
                .with_sprite(SpriteComponent { atlas: AtlasId(0), ..Default::default() }),
            // Objects layer, atlas 1
            Entity::new(EntityId(4))
                .with_layer(RenderLayer::Objects)
                .with_sprite(SpriteComponent { atlas: AtlasId(1), ..Default::default() }),
        ];

        let mut buffer = RenderBuffer::new();
        let batches = build_render_buffer(entities.iter(), &mut buffer);

        assert_eq!(buffer.instance_count(), 4);
        // 4 batches: (Background, atlas 0), (Background, atlas 1), (Objects, atlas 0), (Objects, atlas 1)
        assert_eq!(batches.len(), 4);

        // Background, atlas 0
        assert_eq!(batches[0].layer, RenderLayer::Background);
        assert_eq!(batches[0].atlas_id, 0);
        assert_eq!(batches[0].start, 0);
        assert_eq!(batches[0].end, 1);

        // Background, atlas 1
        assert_eq!(batches[1].layer, RenderLayer::Background);
        assert_eq!(batches[1].atlas_id, 1);
        assert_eq!(batches[1].start, 1);
        assert_eq!(batches[1].end, 2);

        // Objects, atlas 0
        assert_eq!(batches[2].layer, RenderLayer::Objects);
        assert_eq!(batches[2].atlas_id, 0);
        assert_eq!(batches[2].start, 2);
        assert_eq!(batches[2].end, 3);

        // Objects, atlas 1
        assert_eq!(batches[3].layer, RenderLayer::Objects);
        assert_eq!(batches[3].atlas_id, 1);
        assert_eq!(batches[3].start, 3);
        assert_eq!(batches[3].end, 4);

        // Legacy compat: atlas_split = total atlas-0 count (2 entities)
        assert_eq!(buffer.atlas_split, 2);
    }

    #[test]
    fn n_atlas_support() {
        // Test with 4 atlases to verify N-atlas works
        let entities = vec![
            Entity::new(EntityId(1))
                .with_sprite(SpriteComponent { atlas: AtlasId(0), ..Default::default() }),
            Entity::new(EntityId(2))
                .with_sprite(SpriteComponent { atlas: AtlasId(2), ..Default::default() }),
            Entity::new(EntityId(3))
                .with_sprite(SpriteComponent { atlas: AtlasId(1), ..Default::default() }),
            Entity::new(EntityId(4))
                .with_sprite(SpriteComponent { atlas: AtlasId(3), ..Default::default() }),
            Entity::new(EntityId(5))
                .with_sprite(SpriteComponent { atlas: AtlasId(2), ..Default::default() }),
        ];

        let mut buffer = RenderBuffer::new();
        let batches = build_render_buffer(entities.iter(), &mut buffer);

        assert_eq!(buffer.instance_count(), 5);
        // All on Objects layer, 4 different atlases → 4 batches
        assert_eq!(batches.len(), 4);

        // Verify atlas ordering: 0, 1, 2, 3
        assert_eq!(batches[0].atlas_id, 0);
        assert_eq!(batches[0].end - batches[0].start, 1);

        assert_eq!(batches[1].atlas_id, 1);
        assert_eq!(batches[1].end - batches[1].start, 1);

        assert_eq!(batches[2].atlas_id, 2);
        assert_eq!(batches[2].end - batches[2].start, 2); // 2 entities with atlas 2

        assert_eq!(batches[3].atlas_id, 3);
        assert_eq!(batches[3].end - batches[3].start, 1);

        // Legacy compat: only 1 atlas-0 entity
        assert_eq!(buffer.atlas_split, 1);
    }

    #[test]
    fn empty_entities_produces_no_batches() {
        let entities: Vec<Entity> = vec![];
        let mut buffer = RenderBuffer::new();
        let batches = build_render_buffer(entities.iter(), &mut buffer);
        assert_eq!(buffer.instance_count(), 0);
        assert!(batches.is_empty());
    }
}
