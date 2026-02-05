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
    /// Sphere defined by radius. Used for atoms.
    Sphere { radius: f32 },
    /// Capsule (cylinder + hemispherical caps). Used for bonds.
    /// `radius` controls the tube thickness, `half_height` is the cylinder half-length.
    Capsule { radius: f32, half_height: f32 },
    /// Rounded box. Used for labels and indicators.
    /// `radius` is the sphere-trace radius, `half_height` is the box half-height,
    /// `corner_radius` rounds the corners.
    RoundedBox { radius: f32, half_height: f32, corner_radius: f32 },
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

    /// Convenience builder for a sphere mesh.
    pub fn sphere(radius: f32, color: SDFColor) -> Self {
        Self::new(SDFShape::Sphere { radius }, color)
    }

    /// Convenience builder for a capsule mesh (bond between atoms).
    pub fn capsule(radius: f32, half_height: f32, color: SDFColor) -> Self {
        Self::new(SDFShape::Capsule { radius, half_height }, color)
    }

    /// Convenience builder for a rounded box mesh (labels / indicators).
    pub fn rounded_box(radius: f32, half_height: f32, corner_radius: f32, color: SDFColor) -> Self {
        Self::new(SDFShape::RoundedBox { radius, half_height, corner_radius }, color)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_component_sphere_builder() {
        let m = MeshComponent::sphere(10.0, SDFColor::new(1.0, 0.0, 0.0));
        match m.shape {
            SDFShape::Sphere { radius } => assert_eq!(radius, 10.0),
            _ => panic!("Expected Sphere"),
        }
        assert_eq!(m.color.r, 1.0);
        assert_eq!(m.shininess, 32.0); // default
    }

    #[test]
    fn mesh_component_capsule_builder() {
        let m = MeshComponent::capsule(5.0, 20.0, SDFColor::new(0.5, 0.5, 0.5));
        match m.shape {
            SDFShape::Capsule { radius, half_height } => {
                assert_eq!(radius, 5.0);
                assert_eq!(half_height, 20.0);
            }
            _ => panic!("Expected Capsule"),
        }
    }

    #[test]
    fn mesh_component_rounded_box_builder() {
        let m = MeshComponent::rounded_box(15.0, 10.0, 3.0, SDFColor::default())
            .with_shininess(64.0)
            .with_emissive(0.8);
        match m.shape {
            SDFShape::RoundedBox { radius, half_height, corner_radius } => {
                assert_eq!(radius, 15.0);
                assert_eq!(half_height, 10.0);
                assert_eq!(corner_radius, 3.0);
            }
            _ => panic!("Expected RoundedBox"),
        }
        assert_eq!(m.shininess, 64.0);
        assert_eq!(m.emissive, 0.8);
    }
}
