//! Lyon-based vector/polygon rendering system.
//!
//! Provides CPU-side tessellation of filled and stroked shapes using Lyon,
//! producing a flat vertex buffer that gets rendered via WebGPU.
//!
//! # Usage
//!
//! ```ignore
//! // In your Game::update():
//! ctx.vectors.fill_polygon(&[
//!     Vec2::new(100.0, 100.0),
//!     Vec2::new(200.0, 100.0),
//!     Vec2::new(150.0, 200.0),
//! ], VectorColor::RED);
//!
//! ctx.vectors.fill_rect(Vec2::new(300.0, 100.0), 100.0, 50.0, VectorColor::BLUE);
//! ctx.vectors.stroke_polyline(&path_points, 3.0, VectorColor::WHITE);
//! ```

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, FillVertexConstructor,
    StrokeOptions, StrokeTessellator, StrokeVertex, StrokeVertexConstructor, VertexBuffers,
};

/// Per-vertex data for vector/polygon rendering.
/// 6 floats = 24 bytes per vertex.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct VectorVertex {
    pub x: f32,
    pub y: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl VectorVertex {
    /// Number of floats per vertex.
    pub const FLOATS: usize = 6;
    /// Stride in bytes.
    pub const STRIDE_BYTES: usize = Self::FLOATS * 4; // 24
}

/// RGBA color for vector drawing operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl VectorColor {
    /// Create a color from RGBA components (0.0 - 1.0).
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a fully opaque color from RGB components.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create a color from RGB u8 values (0-255) with full opacity.
    pub fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Create a color from RGBA u8 values (0-255).
    pub fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Create a color with the given alpha value.
    pub const fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    // Named color constants
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
    pub const ORANGE: Self = Self::rgb(1.0, 0.5, 0.0);
    pub const PURPLE: Self = Self::rgb(0.5, 0.0, 1.0);
    pub const GRAY: Self = Self::rgb(0.5, 0.5, 0.5);
    pub const DARK_GRAY: Self = Self::rgb(0.25, 0.25, 0.25);
    pub const LIGHT_GRAY: Self = Self::rgb(0.75, 0.75, 0.75);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
}

impl Default for VectorColor {
    fn default() -> Self {
        Self::WHITE
    }
}

/// Vertex constructor for lyon fill tessellation.
struct FillVertexCtor {
    color: VectorColor,
}

impl FillVertexConstructor<VectorVertex> for FillVertexCtor {
    fn new_vertex(&mut self, vertex: FillVertex) -> VectorVertex {
        VectorVertex {
            x: vertex.position().x,
            y: vertex.position().y,
            r: self.color.r,
            g: self.color.g,
            b: self.color.b,
            a: self.color.a,
        }
    }
}

/// Vertex constructor for lyon stroke tessellation.
struct StrokeVertexCtor {
    color: VectorColor,
}

impl StrokeVertexConstructor<VectorVertex> for StrokeVertexCtor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> VectorVertex {
        VectorVertex {
            x: vertex.position().x,
            y: vertex.position().y,
            r: self.color.r,
            g: self.color.g,
            b: self.color.b,
            a: self.color.a,
        }
    }
}

/// State for vector/polygon rendering.
///
/// Holds lyon tessellators and the output vertex buffer.
/// Cleared each frame and populated by drawing commands.
pub struct VectorState {
    fill_tess: FillTessellator,
    stroke_tess: StrokeTessellator,
    geometry: VertexBuffers<VectorVertex, u32>,
    buffer: Vec<f32>,
}

impl VectorState {
    /// Create a new VectorState.
    pub fn new() -> Self {
        Self {
            fill_tess: FillTessellator::new(),
            stroke_tess: StrokeTessellator::new(),
            geometry: VertexBuffers::new(),
            buffer: Vec::with_capacity(16384 * VectorVertex::FLOATS),
        }
    }

    /// Clear the vertex buffer. Called at the start of each frame.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Number of vertices currently in the buffer.
    pub fn vertex_count(&self) -> usize {
        self.buffer.len() / VectorVertex::FLOATS
    }

    /// Raw pointer to the flat float buffer (for SAB copy).
    pub fn buffer_ptr(&self) -> *const f32 {
        self.buffer.as_ptr()
    }

    /// Flush indexed geometry to the flat buffer as triangle list.
    fn flush_geometry(&mut self) {
        for idx in &self.geometry.indices {
            let v = &self.geometry.vertices[*idx as usize];
            self.buffer.extend_from_slice(&[v.x, v.y, v.r, v.g, v.b, v.a]);
        }
        self.geometry.vertices.clear();
        self.geometry.indices.clear();
    }

    /// Tessellate and fill a polygon.
    ///
    /// The polygon is closed automatically. Supports convex and concave shapes.
    pub fn fill_polygon(&mut self, points: &[Vec2], color: VectorColor) {
        if points.len() < 3 {
            return;
        }

        let mut builder = Path::builder();
        builder.begin(point(points[0].x, points[0].y));
        for p in &points[1..] {
            builder.line_to(point(p.x, p.y));
        }
        builder.close();
        let path = builder.build();

        self.fill_path(&path, color);
    }

    /// Tessellate and fill a rectangle.
    pub fn fill_rect(&mut self, pos: Vec2, width: f32, height: f32, color: VectorColor) {
        let points = [
            pos,
            Vec2::new(pos.x + width, pos.y),
            Vec2::new(pos.x + width, pos.y + height),
            Vec2::new(pos.x, pos.y + height),
        ];
        self.fill_polygon(&points, color);
    }

    /// Tessellate and fill a circle.
    ///
    /// The circle is approximated using lyon's default tolerance.
    pub fn fill_circle(&mut self, center: Vec2, radius: f32, color: VectorColor) {
        if radius <= 0.0 {
            return;
        }

        let mut builder = Path::builder();
        builder.add_circle(point(center.x, center.y), radius, lyon::path::Winding::Positive);
        let path = builder.build();

        self.fill_path(&path, color);
    }

    /// Tessellate and fill an ellipse.
    pub fn fill_ellipse(&mut self, center: Vec2, radii: Vec2, color: VectorColor) {
        if radii.x <= 0.0 || radii.y <= 0.0 {
            return;
        }

        let mut builder = Path::builder();
        builder.add_ellipse(
            point(center.x, center.y),
            lyon::math::vector(radii.x, radii.y),
            lyon::math::Angle::radians(0.0),
            lyon::path::Winding::Positive,
        );
        let path = builder.build();

        self.fill_path(&path, color);
    }

    /// Tessellate and fill an arbitrary lyon Path.
    pub fn fill_path(&mut self, path: &Path, color: VectorColor) {
        let result = self.fill_tess.tessellate_path(
            path,
            &FillOptions::tolerance(0.5),
            &mut BuffersBuilder::new(&mut self.geometry, FillVertexCtor { color }),
        );

        if result.is_ok() {
            self.flush_geometry();
        }
    }

    /// Tessellate a stroked polyline (open path).
    pub fn stroke_polyline(&mut self, points: &[Vec2], width: f32, color: VectorColor) {
        if points.len() < 2 {
            return;
        }

        let mut builder = Path::builder();
        builder.begin(point(points[0].x, points[0].y));
        for p in &points[1..] {
            builder.line_to(point(p.x, p.y));
        }
        builder.end(false); // open path

        let path = builder.build();
        self.stroke_path(&path, width, color);
    }

    /// Tessellate a stroked closed polygon.
    pub fn stroke_polygon(&mut self, points: &[Vec2], width: f32, color: VectorColor) {
        if points.len() < 3 {
            return;
        }

        let mut builder = Path::builder();
        builder.begin(point(points[0].x, points[0].y));
        for p in &points[1..] {
            builder.line_to(point(p.x, p.y));
        }
        builder.close();

        let path = builder.build();
        self.stroke_path(&path, width, color);
    }

    /// Tessellate a stroked circle.
    pub fn stroke_circle(&mut self, center: Vec2, radius: f32, width: f32, color: VectorColor) {
        if radius <= 0.0 {
            return;
        }

        let mut builder = Path::builder();
        builder.add_circle(point(center.x, center.y), radius, lyon::path::Winding::Positive);
        let path = builder.build();

        self.stroke_path(&path, width, color);
    }

    /// Tessellate a stroked rectangle.
    pub fn stroke_rect(&mut self, pos: Vec2, width: f32, height: f32, line_width: f32, color: VectorColor) {
        let points = [
            pos,
            Vec2::new(pos.x + width, pos.y),
            Vec2::new(pos.x + width, pos.y + height),
            Vec2::new(pos.x, pos.y + height),
        ];
        self.stroke_polygon(&points, line_width, color);
    }

    /// Tessellate an arbitrary stroked lyon Path.
    pub fn stroke_path(&mut self, path: &Path, width: f32, color: VectorColor) {
        let result = self.stroke_tess.tessellate_path(
            path,
            &StrokeOptions::tolerance(0.5).with_line_width(width),
            &mut BuffersBuilder::new(&mut self.geometry, StrokeVertexCtor { color }),
        );

        if result.is_ok() {
            self.flush_geometry();
        }
    }
}

impl Default for VectorState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn vector_vertex_is_24_bytes() {
        assert_eq!(size_of::<VectorVertex>(), 24);
        assert_eq!(VectorVertex::FLOATS, 6);
        assert_eq!(VectorVertex::STRIDE_BYTES, 24);
    }

    #[test]
    fn vector_color_constructors() {
        let c1 = VectorColor::RED;
        assert_eq!(c1.r, 1.0);
        assert_eq!(c1.g, 0.0);
        assert_eq!(c1.b, 0.0);
        assert_eq!(c1.a, 1.0);

        let c2 = VectorColor::new(0.5, 0.6, 0.7, 0.8);
        assert_eq!(c2.r, 0.5);
        assert_eq!(c2.a, 0.8);

        let c3 = VectorColor::rgb(0.1, 0.2, 0.3);
        assert_eq!(c3.a, 1.0);

        let c4 = VectorColor::rgb8(255, 128, 0);
        assert!((c4.r - 1.0).abs() < 0.01);
        assert!((c4.g - 0.5).abs() < 0.01);
        assert_eq!(c4.b, 0.0);
    }

    #[test]
    fn fill_polygon_triangle() {
        let mut state = VectorState::new();
        let points = [
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(50.0, 100.0),
        ];
        state.fill_polygon(&points, VectorColor::RED);

        // A triangle should produce exactly 3 vertices (1 triangle)
        assert_eq!(state.vertex_count(), 3);
    }

    #[test]
    fn fill_rect_produces_triangles() {
        let mut state = VectorState::new();
        state.fill_rect(Vec2::ZERO, 100.0, 50.0, VectorColor::BLUE);

        // A rectangle should produce 6 vertices (2 triangles)
        assert_eq!(state.vertex_count(), 6);
    }

    #[test]
    fn fill_circle_produces_vertices() {
        let mut state = VectorState::new();
        state.fill_circle(Vec2::new(50.0, 50.0), 25.0, VectorColor::GREEN);

        // Circle produces many triangles (depends on tolerance)
        assert!(state.vertex_count() > 0);
    }

    #[test]
    fn stroke_polyline_produces_vertices() {
        let mut state = VectorState::new();
        let points = [Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0)];
        state.stroke_polyline(&points, 5.0, VectorColor::WHITE);

        // A stroked line produces multiple vertices
        assert!(state.vertex_count() > 0);
    }

    #[test]
    fn clear_resets_buffer() {
        let mut state = VectorState::new();
        state.fill_rect(Vec2::ZERO, 100.0, 50.0, VectorColor::BLUE);
        assert!(state.vertex_count() > 0);

        state.clear();
        assert_eq!(state.vertex_count(), 0);
    }

    #[test]
    fn empty_polygon_produces_nothing() {
        let mut state = VectorState::new();
        state.fill_polygon(&[], VectorColor::RED);
        assert_eq!(state.vertex_count(), 0);

        state.fill_polygon(&[Vec2::ZERO], VectorColor::RED);
        assert_eq!(state.vertex_count(), 0);

        state.fill_polygon(&[Vec2::ZERO, Vec2::ONE], VectorColor::RED);
        assert_eq!(state.vertex_count(), 0);
    }
}
