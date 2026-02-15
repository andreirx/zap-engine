//! Renderer trait for future native GPU backends.
//!
//! Currently, all rendering happens in TypeScript (WebGPU/Canvas2D).
//! This trait defines the contract for future Rust-native renderers
//! (e.g., Metal on iOS/macOS, Vulkan on Android/desktop).
//!
//! The trait mirrors the TypeScript `Renderer` interface in `packages/zap-web/src/renderer/types.ts`.

use super::instance::RenderInstance;
use super::sdf_instance::SDFInstance;
use crate::systems::lighting::PointLight;

/// Render tier indicating GPU capabilities.
/// Mirrors TypeScript `RenderTier` in `packages/zap-web/src/renderer/types.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderTier {
    /// HDR with Extended Dynamic Range (Display-P3, extended tone mapping)
    HdrEdr,
    /// HDR within sRGB gamut (rgba16float but no EDR)
    HdrSrgb,
    /// Standard Dynamic Range (bgra8unorm)
    Sdr,
    /// Software fallback (Canvas 2D / CPU rasterization)
    Software,
}

/// Layer batch descriptor for multi-layer rendering.
/// Mirrors the wire format: 4 floats per batch.
#[derive(Debug, Clone, Copy, Default)]
pub struct LayerBatch {
    /// Layer ID (0=Background through 5=UI)
    pub layer_id: u8,
    /// Start index in the instance array
    pub start: u32,
    /// End index (exclusive) in the instance array
    pub end: u32,
    /// Atlas split within this layer's instances
    pub atlas_split: u32,
}

/// Bake state for layer caching.
#[derive(Debug, Clone, Copy, Default)]
pub struct BakeState {
    /// Bitmask of baked layers (bits 0-5)
    pub mask: u8,
    /// Generation counter for dirty detection
    pub generation: u32,
}

/// Lighting state for dynamic point lights.
#[derive(Debug, Clone)]
pub struct LightingState {
    /// Ambient light RGB (0.0-1.0 each)
    pub ambient: [f32; 3],
    /// Active point lights
    pub lights: Vec<PointLight>,
}

impl Default for LightingState {
    fn default() -> Self {
        Self {
            ambient: [1.0, 1.0, 1.0], // Unlit (full white ambient)
            lights: Vec::new(),
        }
    }
}

/// Timing information from a draw call.
#[derive(Debug, Clone, Copy, Default)]
pub struct DrawTiming {
    /// Time spent submitting draw calls (microseconds)
    pub draw_us: u32,
    /// Time spent in GPU rasterization (microseconds, if measurable)
    pub raster_us: u32,
}

/// Effects vertex for particle/arc rendering.
/// 5 floats per vertex (x, y, color_idx, u, v).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct EffectsVertex {
    pub x: f32,
    pub y: f32,
    /// Color index (encoded as z for the color lookup)
    pub color_idx: f32,
    pub u: f32,
    pub v: f32,
}

/// Vector vertex for polygon/polyline rendering.
/// 6 floats per vertex (x, y, r, g, b, a).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VectorVertex {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Renderer trait for GPU backends.
///
/// Implementors provide platform-specific rendering using the data structures
/// produced by the engine's systems (Scene, Effects, Lighting, etc.).
///
/// # Example Implementation
///
/// ```ignore
/// struct MetalRenderer {
///     device: metal::Device,
///     queue: metal::CommandQueue,
///     // ...
/// }
///
/// impl Renderer for MetalRenderer {
///     fn backend(&self) -> &'static str { "metal" }
///     fn tier(&self) -> RenderTier { RenderTier::HdrEdr }
///
///     fn draw(&mut self, frame: &FrameData) -> DrawTiming {
///         // Encode Metal commands...
///     }
///
///     fn resize(&mut self, width: u32, height: u32) {
///         // Recreate swap chain...
///     }
/// }
/// ```
pub trait Renderer {
    /// Backend identifier (e.g., "webgpu", "metal", "vulkan", "canvas2d")
    fn backend(&self) -> &'static str;

    /// Current render tier based on hardware capabilities
    fn tier(&self) -> RenderTier;

    /// Draw a complete frame with all render data.
    /// Returns timing information for profiling.
    fn draw(&mut self, frame: &FrameData) -> DrawTiming;

    /// Handle window resize. Recreates swap chain and intermediate buffers.
    fn resize(&mut self, width: u32, height: u32);
}

/// Complete frame data for rendering.
/// Aggregates all render data produced by engine systems.
pub struct FrameData<'a> {
    /// Sprite instances (sorted by layer, then atlas)
    pub instances: &'a [RenderInstance],
    /// Layer batch descriptors
    pub layer_batches: &'a [LayerBatch],
    /// SDF molecule instances
    pub sdf_instances: &'a [SDFInstance],
    /// Effects vertices (particles, arcs)
    pub effects_vertices: &'a [EffectsVertex],
    /// Vector vertices (polygons, polylines)
    pub vector_vertices: &'a [VectorVertex],
    /// Layer baking state
    pub bake_state: BakeState,
    /// Dynamic lighting state
    pub lighting: &'a LightingState,
    /// World dimensions for projection matrix
    pub world_width: f32,
    pub world_height: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_tier_variants() {
        assert_ne!(RenderTier::HdrEdr, RenderTier::Sdr);
        assert_eq!(RenderTier::Software, RenderTier::Software);
    }

    #[test]
    fn layer_batch_default() {
        let batch = LayerBatch::default();
        assert_eq!(batch.layer_id, 0);
        assert_eq!(batch.start, 0);
        assert_eq!(batch.end, 0);
    }

    #[test]
    fn lighting_state_default_is_unlit() {
        let state = LightingState::default();
        assert_eq!(state.ambient, [1.0, 1.0, 1.0]);
        assert!(state.lights.is_empty());
    }
}
