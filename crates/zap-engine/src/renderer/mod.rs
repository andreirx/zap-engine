pub mod instance;
pub mod camera;
pub mod sdf_instance;
pub mod traits;

// Re-export key types for convenient access
pub use traits::{
    Renderer, RenderTier, FrameData, DrawTiming,
    LayerBatch, BakeState as RenderBakeState, LightingState,
    EffectsVertex, VectorVertex,
};
