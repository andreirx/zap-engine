//! 3D math primitives for molecule visualization.
//!
//! Provides Vec3 and Camera3D for 3D-to-2D projection without modifying core engine.

use glam::Vec2;
use std::ops::{Add, Sub, Mul, Neg};

/// 3D vector for atom positions and camera math.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const X: Self = Self { x: 1.0, y: 0.0, z: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0, z: 0.0 };
    pub const Z: Self = Self { x: 0.0, y: 0.0, z: 1.0 };

    #[inline]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn splat(v: f32) -> Self {
        Self { x: v, y: v, z: v }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 1e-10 {
            self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Rotate around Y axis (azimuth).
    pub fn rotate_y(self, angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self {
            x: self.x * cos + self.z * sin,
            y: self.y,
            z: -self.x * sin + self.z * cos,
        }
    }

    /// Rotate around X axis (elevation).
    pub fn rotate_x(self, angle: f32) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self {
            x: self.x,
            y: self.y * cos - self.z * sin,
            z: self.y * sin + self.z * cos,
        }
    }

    /// Convert to 2D by dropping z.
    pub fn xy(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;
    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        rhs * self
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

/// Projection result from 3D to 2D.
#[derive(Debug, Clone, Copy)]
pub struct Projection {
    /// 2D screen position.
    pub pos: Vec2,
    /// Depth (positive = closer to camera).
    pub depth: f32,
    /// Scale factor for depth-based sizing.
    pub scale: f32,
}

/// Orbit camera for viewing molecules.
#[derive(Debug, Clone)]
pub struct Camera3D {
    /// Rotation around Y axis (radians).
    pub azimuth: f32,
    /// Rotation around X axis (radians), clamped to avoid gimbal lock.
    pub elevation: f32,
    /// Distance from target point.
    pub distance: f32,
    /// Point the camera looks at (molecule center).
    pub target: Vec3,
    /// Screen dimensions for projection.
    pub screen_width: f32,
    pub screen_height: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            azimuth: 0.0,
            elevation: 0.3, // Slight tilt for better 3D perception
            distance: 300.0,
            target: Vec3::ZERO,
            screen_width: 800.0,
            screen_height: 600.0,
        }
    }
}

impl Camera3D {
    const ORBIT_SENSITIVITY: f32 = 0.008;
    const ZOOM_SPEED: f32 = 0.1;
    const MIN_DISTANCE: f32 = 100.0;
    const MAX_DISTANCE: f32 = 800.0;
    const MAX_ELEVATION: f32 = 1.4; // ~80 degrees

    /// Create camera centered on world origin.
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
            ..Default::default()
        }
    }

    /// Orbit camera by mouse delta.
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.azimuth += dx * Self::ORBIT_SENSITIVITY;
        self.elevation -= dy * Self::ORBIT_SENSITIVITY;
        self.elevation = self.elevation.clamp(-Self::MAX_ELEVATION, Self::MAX_ELEVATION);
    }

    /// Zoom camera (positive = zoom in).
    pub fn zoom(&mut self, delta: f32) {
        self.distance *= 1.0 - delta * Self::ZOOM_SPEED;
        self.distance = self.distance.clamp(Self::MIN_DISTANCE, Self::MAX_DISTANCE);
    }

    /// Pan camera target in world space.
    pub fn pan(&mut self, dx: f32, dy: f32, dz: f32) {
        // Scale pan speed by distance for consistent feel
        let scale = self.distance * 0.002;
        self.target.x += dx * scale;
        self.target.y += dy * scale;
        self.target.z += dz * scale;
    }

    /// Rotate azimuth (around Y axis) by delta radians.
    pub fn rotate_azimuth(&mut self, delta: f32) {
        self.azimuth += delta;
    }

    /// Rotate elevation (around X axis) by delta radians, clamped.
    pub fn rotate_elevation(&mut self, delta: f32) {
        self.elevation += delta;
        self.elevation = self.elevation.clamp(-Self::MAX_ELEVATION, Self::MAX_ELEVATION);
    }

    /// Reset camera to default view.
    pub fn reset(&mut self) {
        self.azimuth = 0.0;
        self.elevation = 0.3;
        self.distance = 300.0;
    }

    /// Update screen dimensions.
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    /// Transform world position to camera-relative view space.
    fn world_to_view(&self, pos: Vec3) -> Vec3 {
        // Translate to target-centered coordinates
        let rel = pos - self.target;

        // Rotate by negative azimuth (around Y)
        let after_y = rel.rotate_y(-self.azimuth);

        // Rotate by negative elevation (around X)
        let after_x = after_y.rotate_x(-self.elevation);

        // Camera is at (0, 0, distance) looking at origin
        Vec3::new(after_x.x, after_x.y, after_x.z - self.distance)
    }

    /// Project 3D world position to 2D screen coordinates.
    pub fn project(&self, pos: Vec3) -> Projection {
        let view = self.world_to_view(pos);

        // Perspective divide (camera looks down -Z, so positive z = behind camera)
        let z_depth = -view.z;
        let safe_depth = z_depth.max(10.0); // Avoid division by zero

        // Perspective scale (larger = closer)
        let fov_scale = self.distance * 0.8; // Approximate FOV effect
        let scale = fov_scale / safe_depth;

        // Screen coordinates (centered)
        let screen_x = self.screen_width / 2.0 + view.x * scale;
        let screen_y = self.screen_height / 2.0 - view.y * scale; // Flip Y for screen coords

        Projection {
            pos: Vec2::new(screen_x, screen_y),
            depth: z_depth,
            scale,
        }
    }

    /// Unproject 2D screen position to 3D ray direction (for hit testing).
    pub fn unproject_ray(&self, screen_pos: Vec2) -> Vec3 {
        let fov_scale = self.distance * 0.8;

        // Reverse the projection math
        let view_x = (screen_pos.x - self.screen_width / 2.0) / fov_scale;
        let view_y = -(screen_pos.y - self.screen_height / 2.0) / fov_scale;

        // Ray direction in view space (pointing into screen)
        let ray_view = Vec3::new(view_x, view_y, -1.0).normalize();

        // Rotate back to world space
        ray_view.rotate_x(self.elevation).rotate_y(self.azimuth)
    }

    /// Get camera position in world space.
    pub fn position(&self) -> Vec3 {
        // Camera is at distance along the viewing direction
        let dir = Vec3::new(0.0, 0.0, self.distance)
            .rotate_x(self.elevation)
            .rotate_y(self.azimuth);
        self.target + dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_dot() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert!((a.dot(b) - 32.0).abs() < 1e-6);
    }

    #[test]
    fn vec3_cross() {
        let x = Vec3::X;
        let y = Vec3::Y;
        let z = x.cross(y);
        assert!((z.x).abs() < 1e-6);
        assert!((z.y).abs() < 1e-6);
        assert!((z.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn vec3_normalize() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 1e-6);
        assert!((n.x - 0.6).abs() < 1e-6);
        assert!((n.y - 0.8).abs() < 1e-6);
    }

    #[test]
    fn vec3_rotate_y() {
        let v = Vec3::X;
        let rotated = v.rotate_y(std::f32::consts::FRAC_PI_2);
        assert!((rotated.x).abs() < 1e-6);
        assert!((rotated.z - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn camera_project_center() {
        let camera = Camera3D::new(800.0, 600.0);
        let proj = camera.project(Vec3::ZERO);
        // Center of screen
        assert!((proj.pos.x - 400.0).abs() < 1.0);
        assert!((proj.pos.y - 300.0).abs() < 1.0);
    }

    #[test]
    fn camera_project_depth_scaling() {
        let mut camera = Camera3D::new(800.0, 600.0);
        camera.target = Vec3::ZERO;

        // Point closer to camera should have larger scale
        let near = camera.project(Vec3::new(0.0, 0.0, 50.0));
        let far = camera.project(Vec3::new(0.0, 0.0, -50.0));
        assert!(near.scale > far.scale);
    }

    #[test]
    fn camera_orbit_clamps_elevation() {
        let mut camera = Camera3D::default();
        camera.orbit(0.0, 10000.0); // Large upward movement
        assert!(camera.elevation <= Camera3D::MAX_ELEVATION);
        assert!(camera.elevation >= -Camera3D::MAX_ELEVATION);
    }

    #[test]
    fn camera_zoom_clamps() {
        let mut camera = Camera3D::default();
        camera.zoom(100.0); // Zoom way in
        assert!(camera.distance >= Camera3D::MIN_DISTANCE);

        camera.zoom(-100.0); // Zoom way out
        assert!(camera.distance <= Camera3D::MAX_DISTANCE);
    }
}
