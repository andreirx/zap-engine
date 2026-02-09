/// Dynamic point light system for 2D lighting.
///
/// Lights are persistent — they stay until explicitly removed.
/// Each frame, the engine serializes active lights to the SAB
/// for the renderer's lighting pass.

use glam::Vec2;

/// A 2D point light with position, color, intensity, radius, and layer mask.
///
/// Wire format (8 floats / 32 bytes):
/// `[x, y, r, g, b, intensity, radius, layer_mask]`
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct PointLight {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub intensity: f32,
    pub radius: f32,
    /// Bitmask of which layers this light affects (bits 0-5).
    /// Default: 0x3F (all layers).
    pub layer_mask: f32,
}

impl PointLight {
    /// Create a new point light at the given position.
    ///
    /// - `pos`: World-space position
    /// - `color`: RGB color (typically [0..1] but can exceed 1.0 for HDR)
    /// - `intensity`: Light strength multiplier
    /// - `radius`: Falloff distance in world units
    pub fn new(pos: Vec2, color: [f32; 3], intensity: f32, radius: f32) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            r: color[0],
            g: color[1],
            b: color[2],
            intensity,
            radius,
            layer_mask: 0x3F as f32, // All 6 layers by default
        }
    }

    /// Set which layers this light affects.
    pub fn with_layer_mask(mut self, mask: u8) -> Self {
        self.layer_mask = mask as f32;
        self
    }

    /// Set the position.
    pub fn with_pos(mut self, pos: Vec2) -> Self {
        self.x = pos.x;
        self.y = pos.y;
        self
    }
}

/// Manages active lights and ambient color for the scene.
///
/// Lights are persistent — add them once and they stay until removed.
/// The ambient color defaults to (1.0, 1.0, 1.0) which produces unlit output
/// when no lights are present.
pub struct LightState {
    lights: Vec<PointLight>,
    ambient: [f32; 3],
}

impl LightState {
    pub fn new() -> Self {
        Self {
            lights: Vec::new(),
            ambient: [1.0, 1.0, 1.0],
        }
    }

    /// Create a LightState with a specific light capacity.
    pub fn with_capacity(max_lights: usize) -> Self {
        Self {
            lights: Vec::with_capacity(max_lights),
            ambient: [1.0, 1.0, 1.0],
        }
    }

    /// Add a point light to the scene.
    pub fn add(&mut self, light: PointLight) {
        self.lights.push(light);
    }

    /// Remove all lights.
    pub fn clear(&mut self) {
        self.lights.clear();
    }

    /// Get an iterator over active lights.
    pub fn iter(&self) -> impl Iterator<Item = &PointLight> {
        self.lights.iter()
    }

    /// Get a mutable iterator over active lights.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PointLight> {
        self.lights.iter_mut()
    }

    /// Remove lights that don't match a predicate.
    pub fn retain<F: FnMut(&PointLight) -> bool>(&mut self, f: F) {
        self.lights.retain(f);
    }

    /// Number of active lights.
    pub fn count(&self) -> usize {
        self.lights.len()
    }

    /// Set the ambient light color (default: white = no darkening).
    /// For a dark scene with point lights, use low values like (0.1, 0.1, 0.15).
    pub fn set_ambient(&mut self, r: f32, g: f32, b: f32) {
        self.ambient = [r, g, b];
    }

    /// Get the ambient color.
    pub fn ambient(&self) -> [f32; 3] {
        self.ambient
    }

    /// Pointer to the lights data for SAB serialization.
    pub fn buffer_ptr(&self) -> *const f32 {
        self.lights.as_ptr() as *const f32
    }
}

impl Default for LightState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::protocol::LIGHT_FLOATS;

    #[test]
    fn point_light_new() {
        let light = PointLight::new(Vec2::new(100.0, 200.0), [1.0, 0.5, 0.0], 2.0, 150.0);
        assert_eq!(light.x, 100.0);
        assert_eq!(light.y, 200.0);
        assert_eq!(light.r, 1.0);
        assert_eq!(light.g, 0.5);
        assert_eq!(light.b, 0.0);
        assert_eq!(light.intensity, 2.0);
        assert_eq!(light.radius, 150.0);
        assert_eq!(light.layer_mask, 63.0); // 0x3F
    }

    #[test]
    fn point_light_with_layer_mask() {
        let light = PointLight::new(Vec2::ZERO, [1.0; 3], 1.0, 50.0)
            .with_layer_mask(0b00_0110); // Terrain + Objects only
        assert_eq!(light.layer_mask, 6.0);
    }

    #[test]
    fn light_state_add_and_count() {
        let mut state = LightState::new();
        assert_eq!(state.count(), 0);

        state.add(PointLight::new(Vec2::ZERO, [1.0; 3], 1.0, 50.0));
        state.add(PointLight::new(Vec2::new(10.0, 20.0), [0.5; 3], 2.0, 100.0));
        assert_eq!(state.count(), 2);
    }

    #[test]
    fn light_state_clear() {
        let mut state = LightState::new();
        state.add(PointLight::new(Vec2::ZERO, [1.0; 3], 1.0, 50.0));
        state.clear();
        assert_eq!(state.count(), 0);
    }

    #[test]
    fn light_state_ambient_default() {
        let state = LightState::new();
        assert_eq!(state.ambient(), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn light_state_set_ambient() {
        let mut state = LightState::new();
        state.set_ambient(0.2, 0.2, 0.25);
        assert_eq!(state.ambient(), [0.2, 0.2, 0.25]);
    }

    #[test]
    fn light_state_retain() {
        let mut state = LightState::new();
        state.add(PointLight::new(Vec2::ZERO, [1.0; 3], 0.5, 50.0));
        state.add(PointLight::new(Vec2::ZERO, [1.0; 3], 2.0, 100.0));
        state.add(PointLight::new(Vec2::ZERO, [1.0; 3], 0.1, 30.0));

        state.retain(|l| l.intensity > 0.3);
        assert_eq!(state.count(), 2);
    }

    #[test]
    fn point_light_is_8_floats() {
        assert_eq!(std::mem::size_of::<PointLight>(), LIGHT_FLOATS * 4);
    }
}
