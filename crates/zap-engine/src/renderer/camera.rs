use bytemuck::{Pod, Zeroable};
use glam::Mat4;

/// Orthographic camera for 2D rendering.
/// Produces a projection matrix mapping world units to clip space.
pub struct Camera2D {
    /// Visible width in world units.
    pub width: f32,
    /// Visible height in world units.
    pub height: f32,
    /// Camera center position in world space.
    pub center: [f32; 2],
}

/// GPU-side uniform data for the camera.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub projection: [[f32; 4]; 4],
}

impl Camera2D {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            center: [0.0, 0.0],
        }
    }

    /// Build an orthographic projection matrix.
    /// Origin at center, Y-up, Z in [0, 1].
    pub fn projection_matrix(&self) -> Mat4 {
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;
        let left = self.center[0] - half_w;
        let right = self.center[0] + half_w;
        let bottom = self.center[1] - half_h;
        let top = self.center[1] + half_h;
        Mat4::orthographic_rh(left, right, bottom, top, 0.0, 1.0)
    }

    pub fn uniform(&self) -> CameraUniform {
        CameraUniform {
            projection: self.projection_matrix().to_cols_array_2d(),
        }
    }

    /// Resize the camera viewport (e.g. on window resize).
    /// Maintains aspect ratio by fitting the game area.
    pub fn resize(
        &mut self,
        viewport_width: f32,
        viewport_height: f32,
        game_width: f32,
        game_height: f32,
    ) {
        let horiz_ratio = viewport_width / game_width;
        let vert_ratio = viewport_height / game_height;
        let scale = horiz_ratio.min(vert_ratio);
        self.width = viewport_width / scale;
        self.height = viewport_height / scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_matrix_is_orthographic() {
        let cam = Camera2D::new(800.0, 600.0);
        let mat = cam.projection_matrix();
        let cols = mat.to_cols_array_2d();
        // Orthographic: cols[3] should be [tx, ty, 0, 1]
        assert!((cols[3][3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn resize_maintains_aspect() {
        let mut cam = Camera2D::new(800.0, 600.0);
        cam.resize(1920.0, 1080.0, 800.0, 600.0);
        let ratio = cam.width / cam.height;
        let expected = 1920.0 / 1080.0;
        assert!((ratio - expected).abs() < 0.01);
    }
}
