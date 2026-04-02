/// Identifies which texture atlas a sprite belongs to.
/// Index into the AssetManifest's atlas list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct AtlasId(pub u32);

/// Blend mode for sprite rendering.
///
/// Ordering: Alpha < Additive. Within a layer, alpha sprites render first
/// (standard compositing), then additive sprites (glow on top).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum BlendMode {
    /// Standard alpha blending (src-alpha, one-minus-src-alpha).
    #[default]
    Alpha = 0,
    /// Additive blending for HDR glow effects (src-alpha, one).
    Additive = 1,
}

impl BlendMode {
    /// Convert to u8 for protocol serialization.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Sprite component — defines how an entity appears visually.
#[derive(Debug, Clone)]
pub struct SpriteComponent {
    /// Which atlas this sprite belongs to.
    pub atlas: AtlasId,
    /// Column in the atlas grid.
    pub col: f32,
    /// Row in the atlas grid.
    pub row: f32,
    /// Number of cells this sprite spans (1.0 = single cell, 2.0 = 2x2 block).
    pub cell_span: f32,
    /// Opacity (0.0 = invisible, 1.0 = opaque, >1.0 for HDR glow).
    pub alpha: f32,
    /// Blend mode for rendering.
    pub blend: BlendMode,
}

impl Default for SpriteComponent {
    fn default() -> Self {
        Self {
            atlas: AtlasId(0),
            col: 0.0,
            row: 0.0,
            cell_span: 1.0,
            alpha: 1.0,
            blend: BlendMode::Alpha,
        }
    }
}
