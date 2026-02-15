use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2};

/// Orthographic camera for 2D rendering.
/// Produces a projection matrix mapping world units to clip space.
pub struct Camera2D {
    /// Visible width in world units.
    pub width: f32,
    /// Visible height in world units.
    pub height: f32,
    /// Camera center position in world space.
    pub center: [f32; 2],
    /// Optional bounds for camera clamping (min_x, min_y, max_x, max_y).
    pub bounds: Option<[f32; 4]>,
    /// Smoothing factor for camera follow (0.0 = instant, 1.0 = never moves).
    pub smoothing: f32,
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
            bounds: None,
            smoothing: 0.0,
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

    /// Set world bounds for camera clamping.
    /// Camera will not show areas outside these bounds.
    pub fn set_bounds(&mut self, min_x: f32, min_y: f32, max_x: f32, max_y: f32) {
        self.bounds = Some([min_x, min_y, max_x, max_y]);
    }

    /// Clear camera bounds (allow camera to move anywhere).
    pub fn clear_bounds(&mut self) {
        self.bounds = None;
    }

    /// Set smoothing factor for camera movement.
    /// 0.0 = instant snap, 0.9 = very smooth/slow.
    pub fn set_smoothing(&mut self, smoothing: f32) {
        self.smoothing = smoothing.clamp(0.0, 0.99);
    }

    /// Move camera center to target position, applying smoothing and bounds.
    pub fn look_at(&mut self, target: Vec2) {
        self.center[0] = target.x;
        self.center[1] = target.y;
        self.clamp_to_bounds();
    }

    /// Smoothly move camera toward target position.
    /// Call this each frame with the target's position.
    pub fn follow(&mut self, target: Vec2, dt: f32) {
        if self.smoothing <= 0.0 {
            // Instant snap
            self.look_at(target);
        } else {
            // Smooth interpolation
            let lerp_factor = 1.0 - self.smoothing.powf(dt * 60.0);
            self.center[0] += (target.x - self.center[0]) * lerp_factor;
            self.center[1] += (target.y - self.center[1]) * lerp_factor;
            self.clamp_to_bounds();
        }
    }

    /// Clamp camera center to bounds if set.
    fn clamp_to_bounds(&mut self) {
        if let Some([min_x, min_y, max_x, max_y]) = self.bounds {
            // Calculate how far the camera center can go
            let half_w = self.width / 2.0;
            let half_h = self.height / 2.0;

            // Clamp center so viewport doesn't go outside bounds
            self.center[0] = self.center[0].clamp(min_x + half_w, max_x - half_w);
            self.center[1] = self.center[1].clamp(min_y + half_h, max_y - half_h);

            // Handle case where viewport is larger than bounds
            let bounds_w = max_x - min_x;
            let bounds_h = max_y - min_y;
            if self.width >= bounds_w {
                self.center[0] = (min_x + max_x) / 2.0;
            }
            if self.height >= bounds_h {
                self.center[1] = (min_y + max_y) / 2.0;
            }
        }
    }

    /// Check if a world-space point is visible in the viewport.
    pub fn is_visible(&self, point: Vec2) -> bool {
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;
        point.x >= self.center[0] - half_w
            && point.x <= self.center[0] + half_w
            && point.y >= self.center[1] - half_h
            && point.y <= self.center[1] + half_h
    }

    /// Check if a world-space rectangle overlaps the viewport.
    pub fn is_rect_visible(&self, rect_center: Vec2, rect_half_size: Vec2) -> bool {
        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        let cam_left = self.center[0] - half_w;
        let cam_right = self.center[0] + half_w;
        let cam_bottom = self.center[1] - half_h;
        let cam_top = self.center[1] + half_h;

        let rect_left = rect_center.x - rect_half_size.x;
        let rect_right = rect_center.x + rect_half_size.x;
        let rect_bottom = rect_center.y - rect_half_size.y;
        let rect_top = rect_center.y + rect_half_size.y;

        rect_right >= cam_left
            && rect_left <= cam_right
            && rect_top >= cam_bottom
            && rect_bottom <= cam_top
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

    #[test]
    fn look_at_moves_camera() {
        let mut cam = Camera2D::new(100.0, 100.0);
        cam.look_at(Vec2::new(500.0, 300.0));
        assert!((cam.center[0] - 500.0).abs() < 1e-6);
        assert!((cam.center[1] - 300.0).abs() < 1e-6);
    }

    #[test]
    fn bounds_clamp_camera() {
        let mut cam = Camera2D::new(100.0, 100.0);
        cam.set_bounds(0.0, 0.0, 500.0, 400.0);

        // Try to look outside bounds - should clamp
        cam.look_at(Vec2::new(0.0, 0.0));
        // Camera center should be at min_x + half_w, min_y + half_h
        assert!((cam.center[0] - 50.0).abs() < 1e-6);
        assert!((cam.center[1] - 50.0).abs() < 1e-6);

        // Try upper bound
        cam.look_at(Vec2::new(1000.0, 1000.0));
        // Should clamp to max_x - half_w, max_y - half_h
        assert!((cam.center[0] - 450.0).abs() < 1e-6);
        assert!((cam.center[1] - 350.0).abs() < 1e-6);
    }

    #[test]
    fn follow_with_no_smoothing_snaps() {
        let mut cam = Camera2D::new(100.0, 100.0);
        cam.set_smoothing(0.0);
        cam.follow(Vec2::new(200.0, 150.0), 0.016);
        assert!((cam.center[0] - 200.0).abs() < 1e-6);
        assert!((cam.center[1] - 150.0).abs() < 1e-6);
    }

    #[test]
    fn follow_with_smoothing_interpolates() {
        let mut cam = Camera2D::new(100.0, 100.0);
        cam.center = [0.0, 0.0];
        cam.set_smoothing(0.9);

        // Follow target at (100, 100)
        cam.follow(Vec2::new(100.0, 100.0), 0.016);

        // Should have moved partway, but not all the way
        assert!(cam.center[0] > 0.0 && cam.center[0] < 100.0);
        assert!(cam.center[1] > 0.0 && cam.center[1] < 100.0);
    }

    #[test]
    fn is_visible_detects_points_in_view() {
        let mut cam = Camera2D::new(100.0, 100.0);
        cam.center = [50.0, 50.0]; // Viewport: [0,100] x [0,100]

        assert!(cam.is_visible(Vec2::new(50.0, 50.0))); // center
        assert!(cam.is_visible(Vec2::new(0.0, 0.0)));   // corner
        assert!(cam.is_visible(Vec2::new(99.0, 99.0))); // near edge
        assert!(!cam.is_visible(Vec2::new(-1.0, 50.0))); // outside left
        assert!(!cam.is_visible(Vec2::new(101.0, 50.0))); // outside right
    }

    #[test]
    fn is_rect_visible_detects_overlap() {
        let mut cam = Camera2D::new(100.0, 100.0);
        cam.center = [50.0, 50.0]; // Viewport: [0,100] x [0,100]

        // Rect fully inside
        assert!(cam.is_rect_visible(Vec2::new(50.0, 50.0), Vec2::new(10.0, 10.0)));

        // Rect partially overlapping
        assert!(cam.is_rect_visible(Vec2::new(-5.0, 50.0), Vec2::new(10.0, 10.0)));

        // Rect fully outside
        assert!(!cam.is_rect_visible(Vec2::new(-50.0, 50.0), Vec2::new(10.0, 10.0)));
    }

    #[test]
    fn clear_bounds_allows_free_movement() {
        let mut cam = Camera2D::new(100.0, 100.0);
        cam.set_bounds(0.0, 0.0, 100.0, 100.0);
        cam.clear_bounds();

        cam.look_at(Vec2::new(-500.0, -500.0));
        assert!((cam.center[0] - -500.0).abs() < 1e-6);
        assert!((cam.center[1] - -500.0).abs() < 1e-6);
    }
}
