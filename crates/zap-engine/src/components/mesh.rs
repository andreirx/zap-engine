/// RGB color for SDF rendering.
#[derive(Debug, Clone, Copy)]
pub struct SDFColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl SDFColor {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

impl Default for SDFColor {
    fn default() -> Self {
        Self { r: 0.6, g: 0.6, b: 0.8 }
    }
}

/// SDF shape primitive.
#[derive(Debug, Clone, Copy)]
pub enum SDFShape {
    Sphere { radius: f32 },
}

/// Component for SDF-rendered meshes (raymarched spheres).
#[derive(Debug, Clone, Copy)]
pub struct MeshComponent {
    pub shape: SDFShape,
    pub color: SDFColor,
    /// Phong specular exponent (default: 32.0).
    pub shininess: f32,
    /// HDR glow multiplier (default: 0.0, values > 0 push into EDR range).
    pub emissive: f32,
}

impl Default for MeshComponent {
    fn default() -> Self {
        Self {
            shape: SDFShape::Sphere { radius: 10.0 },
            color: SDFColor::default(),
            shininess: 32.0,
            emissive: 0.0,
        }
    }
}

impl MeshComponent {
    pub fn new(shape: SDFShape, color: SDFColor) -> Self {
        Self {
            shape,
            color,
            ..Default::default()
        }
    }

    pub fn with_shininess(mut self, shininess: f32) -> Self {
        self.shininess = shininess;
        self
    }

    pub fn with_emissive(mut self, emissive: f32) -> Self {
        self.emissive = emissive;
        self
    }
}
