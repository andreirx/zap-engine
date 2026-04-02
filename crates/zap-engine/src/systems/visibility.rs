//! Generic visibility mask for fog-of-war and region darkening.
//!
//! Games provide a byte grid (0=hidden, 255=fully visible) that maps 1:1 to the
//! game world. The renderer uses it as a fullscreen post-process to darken regions,
//! applied after world content but before UI.

/// Interpolation mode for the visibility mask texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum VisibilityInterpolation {
    /// Sharp cell boundaries (blocky fog).
    #[default]
    Nearest = 0,
    /// Smooth transitions between cells (feathered fog).
    Linear = 1,
}

/// A 2D grid of visibility values.
///
/// Each cell is a u8: 0 = fully hidden (black), 255 = fully visible.
/// The grid maps to the full game world (GameConfig world_width × world_height).
/// Row-major storage: index = y * cols + x.
#[derive(Debug, Clone)]
pub struct VisibilityMask {
    cols: u32,
    rows: u32,
    cells: Vec<u8>,
    pub interpolation: VisibilityInterpolation,
}

impl VisibilityMask {
    /// Create a new visibility mask, fully visible (all 255).
    pub fn new(cols: u32, rows: u32) -> Self {
        Self {
            cols,
            rows,
            cells: vec![255; (cols * rows) as usize],
            interpolation: VisibilityInterpolation::Nearest,
        }
    }

    /// Create a new visibility mask with the given interpolation mode.
    pub fn with_interpolation(cols: u32, rows: u32, interpolation: VisibilityInterpolation) -> Self {
        Self {
            cols,
            rows,
            cells: vec![255; (cols * rows) as usize],
            interpolation,
        }
    }

    /// Grid width in cells.
    pub fn cols(&self) -> u32 {
        self.cols
    }

    /// Grid height in cells.
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Get the visibility value at (x, y). Returns 0 if out of bounds.
    pub fn get(&self, x: u32, y: u32) -> u8 {
        if x >= self.cols || y >= self.rows {
            return 0;
        }
        self.cells[(y * self.cols + x) as usize]
    }

    /// Set the visibility value at (x, y). No-op if out of bounds.
    pub fn set(&mut self, x: u32, y: u32, val: u8) {
        if x < self.cols && y < self.rows {
            self.cells[(y * self.cols + x) as usize] = val;
        }
    }

    /// Fill the entire mask with a single value.
    pub fn fill(&mut self, val: u8) {
        self.cells.fill(val);
    }

    /// Set all cells to 0 (fully hidden).
    pub fn clear(&mut self) {
        self.fill(0);
    }

    /// Set all cells to 255 (fully visible).
    pub fn reveal_all(&mut self) {
        self.fill(255);
    }

    /// Fill a rectangular region with a value.
    pub fn set_rect(&mut self, x: u32, y: u32, w: u32, h: u32, val: u8) {
        let x_end = (x + w).min(self.cols);
        let y_end = (y + h).min(self.rows);
        for cy in y..y_end {
            for cx in x..x_end {
                self.cells[(cy * self.cols + cx) as usize] = val;
            }
        }
    }

    /// Fill a circular region with a value. Uses integer distance check.
    pub fn set_circle(&mut self, cx: f32, cy: f32, radius: f32, val: u8) {
        let r2 = radius * radius;
        let x_min = ((cx - radius).floor() as i32).max(0) as u32;
        let x_max = ((cx + radius).ceil() as i32).min(self.cols as i32) as u32;
        let y_min = ((cy - radius).floor() as i32).max(0) as u32;
        let y_max = ((cy + radius).ceil() as i32).min(self.rows as i32) as u32;
        for y in y_min..y_max {
            for x in x_min..x_max {
                let dx = x as f32 + 0.5 - cx;
                let dy = y as f32 + 0.5 - cy;
                if dx * dx + dy * dy <= r2 {
                    self.cells[(y * self.cols + x) as usize] = val;
                }
            }
        }
    }

    /// Direct byte access for SAB transport.
    pub fn as_bytes(&self) -> &[u8] {
        &self.cells
    }

    /// Total number of bytes in the mask.
    pub fn byte_count(&self) -> usize {
        self.cells.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_mask_is_fully_visible() {
        let mask = VisibilityMask::new(4, 4);
        assert_eq!(mask.get(0, 0), 255);
        assert_eq!(mask.get(3, 3), 255);
    }

    #[test]
    fn set_and_get() {
        let mut mask = VisibilityMask::new(4, 4);
        mask.set(1, 2, 128);
        assert_eq!(mask.get(1, 2), 128);
        assert_eq!(mask.get(0, 0), 255);
    }

    #[test]
    fn out_of_bounds_returns_zero() {
        let mask = VisibilityMask::new(4, 4);
        assert_eq!(mask.get(4, 0), 0);
        assert_eq!(mask.get(0, 4), 0);
        assert_eq!(mask.get(100, 100), 0);
    }

    #[test]
    fn clear_sets_all_hidden() {
        let mut mask = VisibilityMask::new(4, 4);
        mask.clear();
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(mask.get(x, y), 0);
            }
        }
    }

    #[test]
    fn fill_rect() {
        let mut mask = VisibilityMask::new(8, 8);
        mask.clear();
        mask.set_rect(2, 2, 3, 3, 200);
        assert_eq!(mask.get(2, 2), 200);
        assert_eq!(mask.get(4, 4), 200);
        assert_eq!(mask.get(1, 1), 0);
        assert_eq!(mask.get(5, 5), 0);
    }

    #[test]
    fn set_circle() {
        let mut mask = VisibilityMask::new(10, 10);
        mask.clear();
        mask.set_circle(5.0, 5.0, 2.0, 255);
        assert_eq!(mask.get(5, 5), 255); // center
        assert_eq!(mask.get(0, 0), 0);   // far corner
    }

    #[test]
    fn as_bytes_length() {
        let mask = VisibilityMask::new(16, 16);
        assert_eq!(mask.as_bytes().len(), 256);
        assert_eq!(mask.byte_count(), 256);
    }

    #[test]
    fn interpolation_default_is_nearest() {
        let mask = VisibilityMask::new(4, 4);
        assert_eq!(mask.interpolation, VisibilityInterpolation::Nearest);
    }
}
