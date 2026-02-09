//! Triangle strip mesh generation for electric arcs and particles.

use super::segment_color::SegmentColor;

/// Generate triangle strip vertices from a polyline.
/// Output: Vec of [x, y, z, u, v] floats (5 per vertex).
pub fn build_strip_vertices(
    points: &[[f32; 2]],
    width: f32,
    color: SegmentColor,
) -> Vec<f32> {
    if points.len() < 2 {
        return Vec::new();
    }

    let n = points.len();
    let mut verts = Vec::with_capacity((n + 2) * 2 * 5);

    let dir = |a: [f32; 2], b: [f32; 2]| -> ([f32; 2], [f32; 2]) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let d = [dx / len, dy / len];
        let p = [-d[1], d[0]];
        (d, p)
    };

    let color_z = color as u8 as f32;

    let push_pair = |verts: &mut Vec<f32>, center: [f32; 2], perp: [f32; 2], w: f32, v: f32| {
        // Left vertex (u=0)
        verts.extend_from_slice(&[
            center[0] + perp[0] * w,
            center[1] + perp[1] * w,
            color_z,
            0.0,
            v,
        ]);
        // Right vertex (u=1)
        verts.extend_from_slice(&[
            center[0] - perp[0] * w,
            center[1] - perp[1] * w,
            color_z,
            1.0,
            v,
        ]);
    };

    // Start cap
    let (d0, p0) = dir(points[0], points[1]);
    let start_cap = [points[0][0] - d0[0] * width, points[0][1] - d0[1] * width];
    push_pair(&mut verts, start_cap, p0, width, 0.0);

    // First point
    push_pair(&mut verts, points[0], p0, width, 1.0);

    // Middle points
    for i in 1..n - 1 {
        let (_, p_prev) = dir(points[i - 1], points[i]);
        let (_, p_next) = dir(points[i], points[i + 1]);
        let avg = [p_prev[0] + p_next[0], p_prev[1] + p_next[1]];
        let avg_len = (avg[0] * avg[0] + avg[1] * avg[1]).sqrt().max(0.001);
        let perp = [avg[0] / avg_len, avg[1] / avg_len];
        push_pair(&mut verts, points[i], perp, width, 1.0);
    }

    // Last point
    let (d_last, p_last) = dir(points[n - 2], points[n - 1]);
    push_pair(&mut verts, points[n - 1], p_last, width, 1.0);

    // End cap
    let end_cap = [points[n - 1][0] + d_last[0] * width, points[n - 1][1] + d_last[1] * width];
    push_pair(&mut verts, end_cap, p_last, width, 0.0);

    verts
}

/// Convert triangle strip vertices to triangle list (for WebGPU compatibility).
pub fn strip_to_triangles(strip_verts: &[f32], floats_per_vert: usize) -> Vec<f32> {
    let num_verts = strip_verts.len() / floats_per_vert;
    if num_verts < 3 {
        return Vec::new();
    }
    let num_tris = num_verts - 2;
    let mut out = Vec::with_capacity(num_tris * 3 * floats_per_vert);
    for i in 0..num_tris {
        let (a, b, c) = if i % 2 == 0 {
            (i, i + 1, i + 2)
        } else {
            (i + 1, i, i + 2)
        };
        let base_a = a * floats_per_vert;
        let base_b = b * floats_per_vert;
        let base_c = c * floats_per_vert;
        out.extend_from_slice(&strip_verts[base_a..base_a + floats_per_vert]);
        out.extend_from_slice(&strip_verts[base_b..base_b + floats_per_vert]);
        out.extend_from_slice(&strip_verts[base_c..base_c + floats_per_vert]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_vertices_for_simple_line() {
        let points = [[0.0, 0.0], [100.0, 0.0]];
        let verts = build_strip_vertices(&points, 4.0, SegmentColor::Red);
        // 2 points + 2 caps = 4 vertex pairs = 8 vertices * 5 floats
        assert_eq!(verts.len(), 8 * 5);
    }

    #[test]
    fn strip_to_triangles_correct_count() {
        let strip = vec![0.0; 6 * 5]; // 6 verts, 5 floats each
        let tris = strip_to_triangles(&strip, 5);
        assert_eq!(tris.len() / 5, 12); // 4 triangles * 3 verts
    }

    #[test]
    fn empty_points_returns_empty() {
        let verts = build_strip_vertices(&[], 4.0, SegmentColor::Red);
        assert!(verts.is_empty());
    }

    #[test]
    fn single_point_returns_empty() {
        let verts = build_strip_vertices(&[[0.0, 0.0]], 4.0, SegmentColor::Red);
        assert!(verts.is_empty());
    }
}
