use bytemuck::{Pod, Zeroable};

/// Per-instance render data written to SharedArrayBuffer for the TypeScript renderer.
/// Must match the TypeScript protocol: 8 floats = 32 bytes stride.
///
/// The `scale` field is the world-space rendered size in game units.
/// (Games write the actual size, e.g. 50.0 for a 50-unit tile.)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct RenderInstance {
    /// X position in world space.
    pub x: f32,
    /// Y position in world space.
    pub y: f32,
    /// Rotation in radians.
    pub rotation: f32,
    /// World-space rendered size in game units.
    pub scale: f32,
    /// Atlas column (sprite_id after lookup).
    pub sprite_col: f32,
    /// Opacity (0.0 = invisible, 1.0 = opaque, >1.0 for HDR).
    pub alpha: f32,
    /// UV cell span (1.0 = single cell, 2.0 = 2x2 block).
    pub cell_span: f32,
    /// Atlas row.
    pub atlas_row: f32,
}

impl RenderInstance {
    pub const FLOATS: usize = 8;
    pub const STRIDE_BYTES: usize = Self::FLOATS * 4;
}

/// Render buffer containing all sprite instances and metadata.
pub struct RenderBuffer {
    /// Sprite instances to be rendered, ordered by blend mode:
    /// alpha-blended instances first, then additive instances after `atlas_split`.
    pub instances: Vec<RenderInstance>,
    /// Index where the atlas/blend mode split occurs.
    /// Instances [0..atlas_split) use atlas 0 (alpha blend),
    /// instances [atlas_split..] use atlas 1+ or additive blend.
    pub atlas_split: u32,
}

impl RenderBuffer {
    pub fn new() -> Self {
        Self {
            instances: Vec::with_capacity(512),
            atlas_split: 0,
        }
    }

    pub fn clear(&mut self) {
        self.instances.clear();
        self.atlas_split = 0;
    }

    pub fn push(&mut self, instance: RenderInstance) {
        self.instances.push(instance);
    }

    pub fn set_atlas_split(&mut self, split: u32) {
        self.atlas_split = split;
    }

    pub fn instance_count(&self) -> u32 {
        self.instances.len() as u32
    }

    /// Raw pointer to instance data for SharedArrayBuffer reads.
    pub fn instances_ptr(&self) -> *const f32 {
        self.instances.as_ptr() as *const f32
    }
}

impl Default for RenderBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_instance_is_8_floats() {
        assert_eq!(std::mem::size_of::<RenderInstance>(), 32);
        assert_eq!(RenderInstance::FLOATS, 8);
    }

    #[test]
    fn render_buffer_push_and_count() {
        let mut buf = RenderBuffer::new();
        buf.push(RenderInstance::default());
        buf.push(RenderInstance::default());
        assert_eq!(buf.instance_count(), 2);
    }
}
