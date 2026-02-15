//! Tilemap component for efficient 2D tile-based rendering.
//!
//! Tilemaps store a grid of tile indices that reference sprite cells in an atlas.
//! Rendering is optimized with viewport culling - only visible tiles are rendered.

use crate::components::layer::RenderLayer;
use crate::components::sprite::AtlasId;
use crate::renderer::camera::Camera2D;
use crate::renderer::instance::RenderInstance;
use glam::Vec2;

/// A single tile in the tilemap.
/// None represents an empty/transparent tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tile {
    /// Column in the atlas grid (sprite_col).
    pub col: f32,
    /// Row in the atlas grid (atlas_row).
    pub row: f32,
    /// Rotation in radians (0 = no rotation).
    pub rotation: f32,
    /// Opacity (1.0 = opaque).
    pub alpha: f32,
}

impl Tile {
    /// Create a new tile at the given atlas position.
    pub fn new(col: f32, row: f32) -> Self {
        Self {
            col,
            row,
            rotation: 0.0,
            alpha: 1.0,
        }
    }

    /// Create a tile with rotation.
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Create a tile with custom alpha.
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }
}

/// Tilemap component for grid-based rendering.
///
/// Tiles are stored in row-major order: index = y * width + x
#[derive(Debug, Clone)]
pub struct TilemapComponent {
    /// Width of the tilemap in tiles.
    pub width: u32,
    /// Height of the tilemap in tiles.
    pub height: u32,
    /// Size of each tile in world units.
    pub tile_size: f32,
    /// Atlas containing the tile graphics.
    pub atlas: AtlasId,
    /// Render layer for the tilemap.
    pub layer: RenderLayer,
    /// Position of the tilemap's bottom-left corner in world space.
    pub origin: Vec2,
    /// Grid of tiles. None = empty/transparent tile.
    tiles: Vec<Option<Tile>>,
}

impl TilemapComponent {
    /// Create a new empty tilemap.
    pub fn new(width: u32, height: u32, tile_size: f32) -> Self {
        let count = (width * height) as usize;
        Self {
            width,
            height,
            tile_size,
            atlas: AtlasId(0),
            layer: RenderLayer::Terrain,
            origin: Vec2::ZERO,
            tiles: vec![None; count],
        }
    }

    /// Set the atlas for this tilemap.
    pub fn with_atlas(mut self, atlas: AtlasId) -> Self {
        self.atlas = atlas;
        self
    }

    /// Set the render layer.
    pub fn with_layer(mut self, layer: RenderLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Set the world-space origin (bottom-left corner).
    pub fn with_origin(mut self, origin: Vec2) -> Self {
        self.origin = origin;
        self
    }

    /// Get a tile at grid position (x, y).
    pub fn get(&self, x: u32, y: u32) -> Option<&Tile> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.tiles[(y * self.width + x) as usize].as_ref()
    }

    /// Set a tile at grid position (x, y).
    pub fn set(&mut self, x: u32, y: u32, tile: Option<Tile>) {
        if x < self.width && y < self.height {
            self.tiles[(y * self.width + x) as usize] = tile;
        }
    }

    /// Fill a rectangular region with a tile.
    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, tile: Option<Tile>) {
        for ty in y..(y + h).min(self.height) {
            for tx in x..(x + w).min(self.width) {
                self.set(tx, ty, tile);
            }
        }
    }

    /// Clear all tiles.
    pub fn clear(&mut self) {
        self.tiles.fill(None);
    }

    /// World-space bounds of the tilemap.
    pub fn bounds(&self) -> (Vec2, Vec2) {
        let min = self.origin;
        let max = self.origin + Vec2::new(
            self.width as f32 * self.tile_size,
            self.height as f32 * self.tile_size,
        );
        (min, max)
    }

    /// Convert world position to tile grid coordinates.
    pub fn world_to_tile(&self, world_pos: Vec2) -> Option<(u32, u32)> {
        let local = world_pos - self.origin;
        if local.x < 0.0 || local.y < 0.0 {
            return None;
        }
        let tx = (local.x / self.tile_size) as u32;
        let ty = (local.y / self.tile_size) as u32;
        if tx >= self.width || ty >= self.height {
            return None;
        }
        Some((tx, ty))
    }

    /// Convert tile grid coordinates to world position (center of tile).
    pub fn tile_to_world(&self, x: u32, y: u32) -> Vec2 {
        let half = self.tile_size / 2.0;
        self.origin + Vec2::new(
            x as f32 * self.tile_size + half,
            y as f32 * self.tile_size + half,
        )
    }

    /// Build render instances for visible tiles.
    /// Returns instances for tiles within the camera viewport.
    pub fn build_visible_instances(&self, camera: &Camera2D) -> Vec<RenderInstance> {
        let mut instances = Vec::new();

        // Calculate visible tile range
        let half_w = camera.width / 2.0;
        let half_h = camera.height / 2.0;
        let cam_min = Vec2::new(camera.center[0] - half_w, camera.center[1] - half_h);
        let cam_max = Vec2::new(camera.center[0] + half_w, camera.center[1] + half_h);

        // Convert to tile coordinates with padding
        let local_min = cam_min - self.origin;
        let local_max = cam_max - self.origin;

        let min_tx = ((local_min.x / self.tile_size).floor() as i32).max(0) as u32;
        let min_ty = ((local_min.y / self.tile_size).floor() as i32).max(0) as u32;
        let max_tx = ((local_max.x / self.tile_size).ceil() as i32).max(0) as u32;
        let max_ty = ((local_max.y / self.tile_size).ceil() as i32).max(0) as u32;

        let max_tx = max_tx.min(self.width);
        let max_ty = max_ty.min(self.height);

        // Only iterate visible tiles
        for ty in min_ty..max_ty {
            for tx in min_tx..max_tx {
                if let Some(tile) = self.get(tx, ty) {
                    let world_pos = self.tile_to_world(tx, ty);
                    instances.push(RenderInstance {
                        x: world_pos.x,
                        y: world_pos.y,
                        rotation: tile.rotation,
                        scale: self.tile_size,
                        sprite_col: tile.col,
                        alpha: tile.alpha,
                        cell_span: 1.0,
                        atlas_row: tile.row,
                    });
                }
            }
        }

        instances
    }

    /// Build all instances (no culling). Useful for small tilemaps or baking.
    pub fn build_all_instances(&self) -> Vec<RenderInstance> {
        let mut instances = Vec::new();

        for ty in 0..self.height {
            for tx in 0..self.width {
                if let Some(tile) = self.get(tx, ty) {
                    let world_pos = self.tile_to_world(tx, ty);
                    instances.push(RenderInstance {
                        x: world_pos.x,
                        y: world_pos.y,
                        rotation: tile.rotation,
                        scale: self.tile_size,
                        sprite_col: tile.col,
                        alpha: tile.alpha,
                        cell_span: 1.0,
                        atlas_row: tile.row,
                    });
                }
            }
        }

        instances
    }

    /// Count of non-empty tiles.
    pub fn tile_count(&self) -> usize {
        self.tiles.iter().filter(|t| t.is_some()).count()
    }

    /// Total capacity in tiles.
    pub fn capacity(&self) -> usize {
        (self.width * self.height) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tilemap_is_empty() {
        let tm = TilemapComponent::new(10, 10, 32.0);
        assert_eq!(tm.tile_count(), 0);
        assert_eq!(tm.capacity(), 100);
        assert_eq!(tm.width, 10);
        assert_eq!(tm.height, 10);
    }

    #[test]
    fn set_and_get_tile() {
        let mut tm = TilemapComponent::new(5, 5, 16.0);
        let tile = Tile::new(2.0, 3.0);
        tm.set(2, 3, Some(tile));

        let got = tm.get(2, 3).unwrap();
        assert_eq!(got.col, 2.0);
        assert_eq!(got.row, 3.0);
        assert_eq!(tm.tile_count(), 1);
    }

    #[test]
    fn out_of_bounds_returns_none() {
        let tm = TilemapComponent::new(5, 5, 16.0);
        assert!(tm.get(10, 10).is_none());
    }

    #[test]
    fn fill_rect_works() {
        let mut tm = TilemapComponent::new(10, 10, 32.0);
        tm.fill_rect(2, 2, 3, 3, Some(Tile::new(0.0, 0.0)));
        assert_eq!(tm.tile_count(), 9);
    }

    #[test]
    fn world_to_tile_conversion() {
        let tm = TilemapComponent::new(10, 10, 32.0)
            .with_origin(Vec2::new(100.0, 200.0));

        // Center of tile (0,0) is at (116, 216)
        let coords = tm.world_to_tile(Vec2::new(116.0, 216.0));
        assert_eq!(coords, Some((0, 0)));

        // Center of tile (5,5) is at (100 + 5*32 + 16, 200 + 5*32 + 16) = (276, 376)
        let coords = tm.world_to_tile(Vec2::new(276.0, 376.0));
        assert_eq!(coords, Some((5, 5)));

        // Outside tilemap
        assert!(tm.world_to_tile(Vec2::new(50.0, 50.0)).is_none());
    }

    #[test]
    fn tile_to_world_conversion() {
        let tm = TilemapComponent::new(10, 10, 32.0)
            .with_origin(Vec2::new(100.0, 200.0));

        let world = tm.tile_to_world(0, 0);
        // Center of tile (0,0): origin + half_tile = (116, 216)
        assert!((world.x - 116.0).abs() < 0.001);
        assert!((world.y - 216.0).abs() < 0.001);
    }

    #[test]
    fn viewport_culling() {
        let mut tm = TilemapComponent::new(100, 100, 32.0);
        // Fill entire tilemap
        tm.fill_rect(0, 0, 100, 100, Some(Tile::new(0.0, 0.0)));
        assert_eq!(tm.tile_count(), 10000);

        // Camera showing only a small portion
        let mut camera = Camera2D::new(128.0, 128.0); // 4x4 tiles visible
        camera.center = [64.0, 64.0]; // Looking at tile (2,2) area

        let visible = tm.build_visible_instances(&camera);
        // Should be much less than 10000
        assert!(visible.len() < 100);
        assert!(visible.len() >= 16); // At least 4x4 = 16 tiles
    }

    #[test]
    fn bounds_calculation() {
        let tm = TilemapComponent::new(10, 8, 32.0)
            .with_origin(Vec2::new(50.0, 100.0));

        let (min, max) = tm.bounds();
        assert_eq!(min, Vec2::new(50.0, 100.0));
        assert_eq!(max, Vec2::new(50.0 + 320.0, 100.0 + 256.0));
    }

    #[test]
    fn clear_removes_all_tiles() {
        let mut tm = TilemapComponent::new(10, 10, 32.0);
        tm.fill_rect(0, 0, 10, 10, Some(Tile::new(0.0, 0.0)));
        assert_eq!(tm.tile_count(), 100);

        tm.clear();
        assert_eq!(tm.tile_count(), 0);
    }

    #[test]
    fn tile_rotation_and_alpha() {
        let tile = Tile::new(1.0, 2.0)
            .with_rotation(std::f32::consts::PI)
            .with_alpha(0.5);

        assert_eq!(tile.col, 1.0);
        assert_eq!(tile.row, 2.0);
        assert!((tile.rotation - std::f32::consts::PI).abs() < 0.001);
        assert!((tile.alpha - 0.5).abs() < 0.001);
    }
}
