//! Bitmap font text rendering system.
//!
//! Renders text using existing sprite pipeline — each character becomes an Entity
//! with a SpriteComponent pointing to the appropriate glyph in a font atlas.
//!
//! Font atlases are standard sprite atlases with characters laid out in ASCII order,
//! typically 16 columns × 6 rows for printable ASCII (32-127).

use crate::api::types::EntityId;
use crate::components::entity::Entity;
use crate::components::sprite::{AtlasId, BlendMode, SpriteComponent};
use crate::core::scene::Scene;
use glam::Vec2;

/// Configuration for a bitmap font atlas.
///
/// The atlas is a grid of character glyphs laid out in ASCII order,
/// starting from `start_char` (typically 32 = space).
#[derive(Debug, Clone)]
pub struct FontConfig {
    /// Which atlas contains the font glyphs.
    pub atlas: AtlasId,
    /// Number of columns in the font atlas grid.
    pub cols: u32,
    /// Number of rows in the font atlas grid.
    pub rows: u32,
    /// First ASCII code in the atlas (typically 32 = space).
    pub start_char: u8,
    /// Horizontal advance as fraction of character size (e.g., 0.55 for tight, 1.0 for monospace).
    pub spacing: f32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            atlas: AtlasId(1), // Convention: atlas 0 = game sprites, atlas 1 = font
            cols: 16,
            rows: 6,
            start_char: 32, // space
            spacing: 0.55,
        }
    }
}

impl FontConfig {
    /// Create a new font config with the given atlas.
    pub fn new(atlas: AtlasId) -> Self {
        Self {
            atlas,
            ..Default::default()
        }
    }

    /// Set the grid dimensions.
    pub fn with_grid(mut self, cols: u32, rows: u32) -> Self {
        self.cols = cols;
        self.rows = rows;
        self
    }

    /// Set the starting character (ASCII code).
    pub fn with_start_char(mut self, start_char: u8) -> Self {
        self.start_char = start_char;
        self
    }

    /// Set the character spacing.
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

/// Convert an ASCII character to grid coordinates (col, row) in the font atlas.
///
/// Returns `None` if the character is outside the valid range for this font.
pub fn char_to_grid(c: char, font: &FontConfig) -> Option<(f32, f32)> {
    let ascii = c as u32;
    let start = font.start_char as u32;

    if ascii < start {
        return None;
    }

    let index = ascii - start;
    let max_chars = font.cols * font.rows;

    if index >= max_chars {
        return None;
    }

    let col = (index % font.cols) as f32;
    let row = (index / font.cols) as f32;

    Some((col, row))
}

/// Build a vector of character entities for the given text.
///
/// Each printable character becomes an Entity with a SpriteComponent.
/// Characters outside the font's range are skipped.
///
/// # Arguments
/// * `text` - The text string to render
/// * `pos` - Position of the first character (top-left corner)
/// * `size` - Size of each character in world units
/// * `font` - Font configuration
/// * `tag` - Tag to assign to all character entities (for batch despawn)
/// * `id_gen` - Closure that generates unique EntityIds
pub fn build_text_entities<F>(
    text: &str,
    pos: Vec2,
    size: f32,
    font: &FontConfig,
    tag: &str,
    id_gen: &mut F,
) -> Vec<Entity>
where
    F: FnMut() -> EntityId,
{
    let mut entities = Vec::new();
    let mut cursor_x = pos.x;

    for c in text.chars() {
        if let Some((col, row)) = char_to_grid(c, font) {
            let id = id_gen();
            let entity = Entity::new(id)
                .with_tag(tag)
                .with_pos(Vec2::new(cursor_x + size / 2.0, pos.y + size / 2.0))
                .with_scale(Vec2::splat(size))
                .with_sprite(SpriteComponent {
                    atlas: font.atlas,
                    col,
                    row,
                    cell_span: 1.0,
                    alpha: 1.0,
                    blend: BlendMode::Alpha,
                });
            entities.push(entity);
        }
        // Always advance cursor (even for skipped chars, to preserve spacing)
        cursor_x += size * font.spacing;
    }

    entities
}

/// Despawn all entities with the given tag.
///
/// Useful for removing text that was spawned with a shared tag.
pub fn despawn_text(scene: &mut Scene, tag: &str) {
    // Collect IDs first to avoid borrow conflict
    let ids: Vec<EntityId> = scene
        .iter()
        .filter(|e| e.tag == tag)
        .map(|e| e.id)
        .collect();

    for id in ids {
        scene.despawn(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_default_font() -> FontConfig {
        FontConfig {
            atlas: AtlasId(1),
            cols: 16,
            rows: 6,
            start_char: 32,
            spacing: 0.55,
        }
    }

    #[test]
    fn char_to_grid_basic() {
        let font = make_default_font();
        // 'A' is ASCII 65, start_char is 32, so index = 33
        // col = 33 % 16 = 1, row = 33 / 16 = 2
        let (col, row) = char_to_grid('A', &font).unwrap();
        assert_eq!(col, 1.0);
        assert_eq!(row, 2.0);
    }

    #[test]
    fn char_to_grid_space() {
        let font = make_default_font();
        // ' ' is ASCII 32, start_char is 32, so index = 0
        // col = 0, row = 0
        let (col, row) = char_to_grid(' ', &font).unwrap();
        assert_eq!(col, 0.0);
        assert_eq!(row, 0.0);
    }

    #[test]
    fn char_to_grid_out_of_range() {
        let font = make_default_font();
        // ASCII 31 is below start_char 32
        assert!(char_to_grid('\x1F', &font).is_none());
        // Tab (9) is also below
        assert!(char_to_grid('\t', &font).is_none());
    }

    #[test]
    fn char_to_grid_end_of_range() {
        let font = make_default_font();
        // '~' is ASCII 126, index = 94
        // col = 94 % 16 = 14, row = 94 / 16 = 5
        let (col, row) = char_to_grid('~', &font).unwrap();
        assert_eq!(col, 14.0);
        assert_eq!(row, 5.0);
    }

    #[test]
    fn char_to_grid_beyond_atlas() {
        let font = make_default_font();
        // DEL (127) would be index 95, which is within 16*6=96
        let result = char_to_grid('\x7F', &font);
        assert!(result.is_some()); // 95 < 96, so it's valid

        // But 128 would be index 96, which is >= 96
        let result = char_to_grid('\u{80}', &font);
        assert!(result.is_none());
    }

    #[test]
    fn spawn_text_basic() {
        let font = make_default_font();
        let mut next_id = 1u32;
        let entities = build_text_entities("Hi", Vec2::ZERO, 20.0, &font, "test_text", &mut || {
            let id = EntityId(next_id);
            next_id += 1;
            id
        });

        assert_eq!(entities.len(), 2);

        // 'H' is ASCII 72, index = 40, col = 8, row = 2
        let h = &entities[0];
        assert_eq!(h.tag, "test_text");
        let h_sprite = h.sprite.as_ref().unwrap();
        assert_eq!(h_sprite.col, 8.0);
        assert_eq!(h_sprite.row, 2.0);

        // 'i' is ASCII 105, index = 73, col = 9, row = 4
        let i = &entities[1];
        let i_sprite = i.sprite.as_ref().unwrap();
        assert_eq!(i_sprite.col, 9.0);
        assert_eq!(i_sprite.row, 4.0);
    }

    #[test]
    fn spawn_text_skips_unprintable() {
        let font = make_default_font();
        let mut next_id = 1u32;
        // Tab and newline should be skipped but cursor still advances
        let entities = build_text_entities("A\tB\nC", Vec2::ZERO, 20.0, &font, "test", &mut || {
            let id = EntityId(next_id);
            next_id += 1;
            id
        });

        // Only A, B, C should create entities
        assert_eq!(entities.len(), 3);
    }

    #[test]
    fn despawn_text_removes_tagged() {
        let mut scene = Scene::new();

        // Spawn some entities with tags
        scene.spawn(Entity::new(EntityId(1)).with_tag("text1"));
        scene.spawn(Entity::new(EntityId(2)).with_tag("text1"));
        scene.spawn(Entity::new(EntityId(3)).with_tag("text2"));

        assert_eq!(scene.len(), 3);

        despawn_text(&mut scene, "text1");

        assert_eq!(scene.len(), 1);
        assert!(scene.get(EntityId(3)).is_some());
        assert!(scene.get(EntityId(1)).is_none());
        assert!(scene.get(EntityId(2)).is_none());
    }
}
