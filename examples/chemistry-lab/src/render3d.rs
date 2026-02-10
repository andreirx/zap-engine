//! 3D to 2D rendering with depth sorting.
//!
//! Projects 3D molecules to 2D SDFInstances with perspective and z-sorting.

use crate::math3d::{Camera3D, Vec3};
use crate::molecule3d::MoleculeState3D;
use crate::periodic_table::ElementRegistry;
use zap_engine::renderer::sdf_instance::SDFInstance;
use std::f32::consts::FRAC_PI_2;

/// Render item with depth for sorting.
struct RenderItem {
    depth: f32,
    instance: SDFInstance,
}

/// Bond rendering constants.
const BOND_RADIUS: f32 = 5.0;
const BOND_SHININESS: f32 = 16.0;
const BOND_COLOR: (f32, f32, f32) = (0.4, 0.4, 0.45);

/// Build z-sorted SDF instances from 3D molecule state.
///
/// Projects all atoms and bonds to 2D, applies depth-based sizing,
/// and sorts back-to-front (painter's algorithm).
pub fn build_3d_sdf_buffer(
    molecule: &MoleculeState3D,
    camera: &Camera3D,
    registry: &ElementRegistry,
) -> Vec<SDFInstance> {
    let mut items: Vec<RenderItem> = Vec::with_capacity(
        molecule.atoms.len() + molecule.bonds.len()
    );

    // VISUAL_SCALE: Render atoms at 60% of physical radius to create "Ball & Stick" look.
    // This ensures bonds are visible even when atoms are physically touching.
    const VISUAL_SCALE: f32 = 0.6;

    // Project atoms (spheres)
    for atom in &molecule.atoms {
        let elem = match registry.get(atom.element_kind) {
            Some(e) => e,
            None => continue,
        };

        let proj = camera.project(atom.position);

        items.push(RenderItem {
            depth: proj.depth,
            instance: SDFInstance {
                x: proj.pos.x,
                y: proj.pos.y,
                radius: elem.radius * proj.scale * VISUAL_SCALE, // Scale down visual
                rotation: 0.0,
                r: elem.color.r,
                g: elem.color.g,
                b: elem.color.b,
                shininess: 32.0,
                emissive: 0.0,
                shape_type: 0.0, // Sphere
                half_height: 0.0,
                extra: 0.0,
            },
        });
    }

    // Project bonds (capsules connecting atom surfaces)
    for bond in &molecule.bonds {
        let atom_a = match molecule.atoms.get(bond.atom_a) {
            Some(a) => a,
            None => continue,
        };
        let atom_b = match molecule.atoms.get(bond.atom_b) {
            Some(a) => a,
            None => continue,
        };

        // Get atom covalent radii (in 3D world units)
        let radius_a = registry.get(atom_a.element_kind).map_or(20.0, |e| e.radius);
        let radius_b = registry.get(atom_b.element_kind).map_or(20.0, |e| e.radius);

        // Compute bond direction and length in 3D
        let bond_vec = atom_b.position - atom_a.position;
        let bond_length_3d = bond_vec.length();

        if bond_length_3d < 0.001 {
            continue; // Atoms at same 3D position, skip
        }

        let bond_dir = bond_vec * (1.0 / bond_length_3d); // Normalized direction in 3D

        // Use VISUAL radius for bond attachment points
        let visual_radius_a = radius_a * VISUAL_SCALE;
        let visual_radius_b = radius_b * VISUAL_SCALE;

        // Compute endpoints at the **visual surface**
        let endpoint_a_3d = atom_a.position + bond_dir * visual_radius_a;
        let endpoint_b_3d = atom_b.position - bond_dir * visual_radius_b;

        // Compute 3D distance between visual surfaces (before projection)
        let surface_distance_3d = (endpoint_b_3d - endpoint_a_3d).length();

        // Skip if atoms are essentially overlapping in 3D (shouldn't happen with VISUAL_SCALE)
        if surface_distance_3d < 1.0 {
            continue;
        }

        // Project both endpoints to 2D screen space
        let proj_endpoint_a = camera.project(endpoint_a_3d);
        let proj_endpoint_b = camera.project(endpoint_b_3d);

        // Now compute capsule geometry from the projected endpoints
        let dx = proj_endpoint_b.pos.x - proj_endpoint_a.pos.x;
        let dy = proj_endpoint_b.pos.y - proj_endpoint_a.pos.y;
        let surface_distance_2d = (dx * dx + dy * dy).sqrt();

        // Midpoint in 2D (this is the capsule center)
        let mid_x = (proj_endpoint_a.pos.x + proj_endpoint_b.pos.x) / 2.0;
        let mid_y = (proj_endpoint_a.pos.y + proj_endpoint_b.pos.y) / 2.0;
        let mid_depth = (proj_endpoint_a.depth + proj_endpoint_b.depth) / 2.0;
        let mid_scale = (proj_endpoint_a.scale + proj_endpoint_b.scale) / 2.0;

        // Rotation angle (capsule default orientation is vertical)
        let rotation = dy.atan2(dx) - FRAC_PI_2;

        // Bond tube radius in screen space
        let bond_radius_screen = BOND_RADIUS * mid_scale;

        // Capsule half_height: the cylinder portion length
        // For nearly end-on views, use a minimum half_height based on 3D length
        let min_half_height = surface_distance_3d * mid_scale * 0.25; // 25% of 3D length
        let half_height = (surface_distance_2d / 2.0 - bond_radius_screen)
            .max(min_half_height)
            .max(1.0); // Minimum 1 pixel

        // Only skip if BOTH 3D distance is large AND 2D is very small (true foreshortening)
        // This prevents bonds from disappearing when atoms are correctly positioned
        let foreshortening_ratio = surface_distance_2d / (surface_distance_3d * mid_scale + 0.001);
        if foreshortening_ratio < 0.1 && surface_distance_3d > 20.0 {
            // Viewed nearly perfectly end-on AND bond is long - skip to avoid visual artifacts
            continue;
        }

        items.push(RenderItem {
            depth: mid_depth,
            instance: SDFInstance {
                x: mid_x,
                y: mid_y,
                radius: bond_radius_screen,
                rotation,
                r: BOND_COLOR.0,
                g: BOND_COLOR.1,
                b: BOND_COLOR.2,
                shininess: BOND_SHININESS,
                emissive: 0.0,
                shape_type: 1.0, // Capsule
                half_height,
                extra: 0.0,
            },
        });
    }

    // Sort back-to-front (larger depth = farther = render first)
    items.sort_by(|a, b| {
        // Descending order: farther objects (larger depth) rendered first
        b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal)
    });

    items.into_iter().map(|r| r.instance).collect()
}

/// Hit test in 2D screen space, returning atom index if hit.
pub fn hit_test_2d(
    molecule: &MoleculeState3D,
    camera: &Camera3D,
    registry: &ElementRegistry,
    screen_pos: glam::Vec2,
) -> Option<usize> {
    // Check atoms from front to back (closest first)
    let mut hits: Vec<(usize, f32, f32)> = Vec::new(); // (idx, depth, distance)

    for (idx, atom) in molecule.atoms.iter().enumerate() {
        let elem = match registry.get(atom.element_kind) {
            Some(e) => e,
            None => continue,
        };

        let proj = camera.project(atom.position);
        let screen_radius = elem.radius * proj.scale * 1.5; // Generous hit area

        let dx = screen_pos.x - proj.pos.x;
        let dy = screen_pos.y - proj.pos.y;
        let dist_sq = dx * dx + dy * dy;

        if dist_sq < screen_radius * screen_radius {
            hits.push((idx, proj.depth, dist_sq.sqrt()));
        }
    }

    // Return frontmost hit (highest depth = closest to camera)
    hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.first().map(|(idx, _, _)| *idx)
}

/// Convert 2D screen click to 3D world position on a plane at target depth.
pub fn screen_to_world_on_plane(
    camera: &Camera3D,
    screen_pos: glam::Vec2,
    _plane_z: f32,
) -> Vec3 {
    // Place atom at camera target depth on the view plane
    let fov_scale = camera.distance * 0.8;
    let view_x = (screen_pos.x - camera.screen_width / 2.0) / fov_scale;
    let view_y = -(screen_pos.y - camera.screen_height / 2.0) / fov_scale;

    // Position in view space at target distance
    let view_pos = Vec3::new(view_x * camera.distance, view_y * camera.distance, 0.0);

    // Rotate to world space
    view_pos
        .rotate_x(camera.elevation)
        .rotate_y(camera.azimuth)
        + camera.target
}

#[cfg(test)]
mod tests {
    use super::*;
    use zap_engine::EntityId;

    fn mock_registry() -> ElementRegistry {
        ElementRegistry::load().expect("Failed to load periodic table")
    }

    #[test]
    fn build_buffer_empty() {
        let mol = MoleculeState3D::new();
        let camera = Camera3D::new(800.0, 600.0);
        let registry = mock_registry();

        let buffer = build_3d_sdf_buffer(&mol, &camera, &registry);
        assert!(buffer.is_empty());
    }

    #[test]
    fn build_buffer_single_atom() {
        let mut mol = MoleculeState3D::new();
        mol.add_atom(EntityId(1), 6, Vec3::ZERO); // Carbon

        let camera = Camera3D::new(800.0, 600.0);
        let registry = mock_registry();

        let buffer = build_3d_sdf_buffer(&mol, &camera, &registry);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0].shape_type, 0.0); // Sphere
    }

    #[test]
    fn build_buffer_with_bond() {
        let mut mol = MoleculeState3D::new();
        mol.add_atom(EntityId(1), 6, Vec3::new(-50.0, 0.0, 0.0)); // Carbon
        mol.add_atom(EntityId(2), 6, Vec3::new(50.0, 0.0, 0.0));  // Carbon
        mol.add_bond(0, 1);

        let camera = Camera3D::new(800.0, 600.0);
        let registry = mock_registry();

        let buffer = build_3d_sdf_buffer(&mol, &camera, &registry);
        assert_eq!(buffer.len(), 3); // 2 atoms + 1 bond
    }

    #[test]
    fn depth_sorting() {
        let mut mol = MoleculeState3D::new();
        // Atom closer to camera (positive Z in default view)
        mol.add_atom(EntityId(1), 1, Vec3::new(0.0, 0.0, 50.0));
        // Atom farther from camera
        mol.add_atom(EntityId(2), 1, Vec3::new(0.0, 0.0, -50.0));

        let camera = Camera3D::new(800.0, 600.0);
        let registry = mock_registry();

        let buffer = build_3d_sdf_buffer(&mol, &camera, &registry);
        assert_eq!(buffer.len(), 2);

        // First in buffer should be the farther atom (rendered first)
        // Its radius should be smaller due to depth scaling
        assert!(buffer[0].radius < buffer[1].radius);
    }

    #[test]
    fn hit_test_finds_atom() {
        let mut mol = MoleculeState3D::new();
        mol.add_atom(EntityId(1), 6, Vec3::ZERO);

        let camera = Camera3D::new(800.0, 600.0);
        let registry = mock_registry();

        // Click at screen center (where atom projects to)
        let hit = hit_test_2d(&mol, &camera, &registry, glam::Vec2::new(400.0, 300.0));
        assert_eq!(hit, Some(0));

        // Click far away
        let miss = hit_test_2d(&mol, &camera, &registry, glam::Vec2::new(0.0, 0.0));
        assert_eq!(miss, None);
    }
}
