use crate::components::entity::Entity;
use crate::components::layer::RenderLayer;
use crate::renderer::instance::{RenderBuffer, RenderInstance};

/// Describes a contiguous batch of instances sharing the same layer.
/// Within a batch, instances are grouped by atlas: atlas 0 first, then atlas 1+.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerBatch {
    /// Which render layer this batch belongs to.
    pub layer: RenderLayer,
    /// Start index (inclusive) in the render buffer.
    pub start: u32,
    /// End index (exclusive) in the render buffer.
    pub end: u32,
    /// Index within this batch where atlas 0 ends and atlas 1+ begins.
    /// Relative to the buffer start (NOT relative to batch start).
    pub atlas_split: u32,
}

impl LayerBatch {
    /// Floats per LayerBatch in the protocol wire format.
    pub const FLOATS: usize = 4;
}

/// Build the render buffer from a set of entities.
/// Sorts entities by (layer, atlas) for layered rendering.
/// Returns layer batch descriptors and sets the legacy `atlas_split` for backward compat.
///
/// Draw order: layers back-to-front, within each layer atlas 0 first then atlas 1+.
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

    // Sort by (layer, atlas_bucket) — atlas 0 before atlas 1+
    entries.sort_by(|a, b| {
        a.layer.cmp(&b.layer)
            .then_with(|| {
                // Atlas 0 first, then everything else (treated as one bucket)
                let a_bucket = if a.atlas == 0 { 0u8 } else { 1 };
                let b_bucket = if b.atlas == 0 { 0u8 } else { 1 };
                a_bucket.cmp(&b_bucket)
            })
    });

    // Build buffer and extract batch boundaries
    let mut batches: Vec<LayerBatch> = Vec::new();
    let mut current_layer: Option<RenderLayer> = None;
    let mut batch_start: u32 = 0;
    let mut atlas0_end: u32 = 0;

    for entry in &entries {
        let idx = buffer.instance_count();

        if current_layer != Some(entry.layer) {
            // Close previous batch
            if let Some(layer) = current_layer {
                batches.push(LayerBatch {
                    layer,
                    start: batch_start,
                    end: idx,
                    atlas_split: atlas0_end,
                });
            }
            // Start new batch
            current_layer = Some(entry.layer);
            batch_start = idx;
            atlas0_end = idx; // Will advance as we see atlas 0 entries
        }

        if entry.atlas == 0 {
            // atlas0_end tracks one past the last atlas-0 entry in this batch
            atlas0_end = idx + 1;
        }

        buffer.push(entry.instance);
    }

    // Close final batch
    if let Some(layer) = current_layer {
        batches.push(LayerBatch {
            layer,
            start: batch_start,
            end: buffer.instance_count(),
            atlas_split: atlas0_end,
        });
    }

    // Set legacy atlas_split for backward compatibility:
    // If there's exactly one batch (all Objects, the default), use its atlas_split.
    // Otherwise, set to total instance count (renderer uses batch data instead).
    if batches.len() == 1 {
        buffer.set_atlas_split(batches[0].atlas_split);
    } else if batches.is_empty() {
        buffer.set_atlas_split(0);
    } else {
        // Multi-layer: legacy split is the total atlas-0 count across all layers
        let total_atlas0: u32 = batches.iter().map(|b| b.atlas_split - b.start).sum();
        buffer.set_atlas_split(total_atlas0);
    }

    batches
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
        let batches = build_render_buffer(entities.iter(), &mut buffer);

        assert_eq!(buffer.instance_count(), 4);
        // All entities are on default Objects layer → single batch
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].layer, RenderLayer::Objects);
        assert_eq!(batches[0].start, 0);
        assert_eq!(batches[0].end, 4);
        // Atlas 0 sprites (entities 1 and 3) go first, atlas 1 sprites (2 and 4) go after
        assert_eq!(batches[0].atlas_split, 2);
        assert_eq!(buffer.atlas_split, 2); // legacy compat
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
        assert_eq!(batches.len(), 2);

        // Background batch: atlas 0 at index 0, atlas 1 at index 1
        assert_eq!(batches[0].layer, RenderLayer::Background);
        assert_eq!(batches[0].start, 0);
        assert_eq!(batches[0].end, 2);
        assert_eq!(batches[0].atlas_split, 1); // 1 atlas-0 entity

        // Objects batch: atlas 0 at index 2, atlas 1 at index 3
        assert_eq!(batches[1].layer, RenderLayer::Objects);
        assert_eq!(batches[1].start, 2);
        assert_eq!(batches[1].end, 4);
        assert_eq!(batches[1].atlas_split, 3); // atlas 0 ends at index 3
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
