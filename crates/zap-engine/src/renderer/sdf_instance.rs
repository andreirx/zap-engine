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
    /// SDF shape type: 0.0 = Sphere, 1.0 = Capsule, 2.0 = RoundedBox.
    pub shape_type: f32,
    /// Cylinder half-length (Capsule) or box half-height (RoundedBox). 0.0 for Sphere.
    pub half_height: f32,
    /// Corner radius (RoundedBox only). 0.0 for Sphere/Capsule.
    pub extra: f32,
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

    #[test]
    fn sdf_instance_capsule_encoding() {
        let inst = SDFInstance {
            x: 10.0,
            y: 20.0,
            radius: 5.0,
            rotation: 1.57,
            r: 0.0,
            g: 1.0,
            b: 0.0,
            shininess: 32.0,
            emissive: 0.0,
            shape_type: 1.0,
            half_height: 15.0,
            extra: 0.0,
        };
        let floats: &[f32; 12] = bytemuck::cast_ref(&inst);
        assert_eq!(floats[9], 1.0);  // shape_type at offset 9
        assert_eq!(floats[10], 15.0); // half_height at offset 10
        assert_eq!(floats[11], 0.0);  // extra at offset 11
    }

    #[test]
    fn sdf_instance_rounded_box_encoding() {
        let inst = SDFInstance {
            x: 0.0,
            y: 0.0,
            radius: 20.0,
            rotation: 0.0,
            r: 1.0,
            g: 1.0,
            b: 1.0,
            shininess: 16.0,
            emissive: 0.5,
            shape_type: 2.0,
            half_height: 10.0,
            extra: 3.0,
        };
        let floats: &[f32; 12] = bytemuck::cast_ref(&inst);
        assert_eq!(floats[9], 2.0);  // shape_type
        assert_eq!(floats[10], 10.0); // half_height
        assert_eq!(floats[11], 3.0);  // extra (corner_radius)
    }
}
