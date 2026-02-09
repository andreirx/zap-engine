//! Debug line for visualizing colliders, paths, etc.

use super::segment_color::SegmentColor;

/// A debug line for visualizing colliders, paths, etc.
#[derive(Debug, Clone)]
pub struct DebugLine {
    pub points: Vec<[f32; 2]>,
    pub width: f32,
    pub color: SegmentColor,
}

impl DebugLine {
    /// Create a new debug line.
    pub fn new(points: Vec<[f32; 2]>, width: f32, color: SegmentColor) -> Self {
        Self { points, width, color }
    }
}
