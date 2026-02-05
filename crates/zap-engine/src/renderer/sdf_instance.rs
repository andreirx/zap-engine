use bytemuck::{Pod, Zeroable};

/// Per-instance SDF render data for the molecule pipeline.
/// Written to SharedArrayBuffer for the TypeScript SDF renderer.
/// 12 floats = 48 bytes per instance.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct SDFInstance {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub rotation: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub shininess: f32,
    pub emissive: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

impl SDFInstance {
    pub const FLOATS: usize = 12;
    pub const STRIDE_BYTES: usize = Self::FLOATS * 4;
}

/// Buffer of SDF instances for the molecule rendering pipeline.
pub struct SDFBuffer {
    instances: Vec<SDFInstance>,
}

impl SDFBuffer {
    pub fn new() -> Self {
        Self::with_capacity(128)
    }

    pub fn with_capacity(max: usize) -> Self {
        Self {
            instances: Vec::with_capacity(max),
        }
    }

    pub fn clear(&mut self) {
        self.instances.clear();
    }

    pub fn push(&mut self, instance: SDFInstance) {
        self.instances.push(instance);
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    pub fn instances_ptr(&self) -> *const f32 {
        self.instances.as_ptr() as *const f32
    }
}

impl Default for SDFBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdf_instance_is_48_bytes() {
        assert_eq!(std::mem::size_of::<SDFInstance>(), 48);
        assert_eq!(SDFInstance::FLOATS, 12);
    }

    #[test]
    fn sdf_buffer_push_and_count() {
        let mut buf = SDFBuffer::new();
        buf.push(SDFInstance::default());
        buf.push(SDFInstance::default());
        assert_eq!(buf.instance_count(), 2);
    }
}
