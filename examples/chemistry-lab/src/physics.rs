//! Force-directed physics solver for molecular simulation.
//!
//! Implements spring forces for bonds, angle constraints for VSEPR geometry,
//! and electrostatic repulsion to prevent atom overlap.

use crate::math3d::Vec3;
use crate::molecule3d::MoleculeState3D;
use crate::periodic_table::ElementRegistry;
use crate::vsepr::{calc_lone_pairs, VSEPRGeometry};

/// Physics simulation parameters.
#[derive(Debug, Clone)]
pub struct PhysicsConfig {
    /// Spring constant for bond length maintenance (N/unit).
    pub bond_stiffness: f32,
    /// Spring constant for angle constraints (N*unit/rad).
    pub angle_stiffness: f32,
    /// Repulsion constant for electrostatic-like forces.
    pub repulsion_strength: f32,
    /// Minimum distance for repulsion calculation (prevents division by zero).
    pub repulsion_min_dist: f32,
    /// Damping factor to prevent oscillation (0-1).
    pub damping: f32,
    /// Maximum force magnitude (prevents instability).
    pub max_force: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            bond_stiffness: 200.0,
            angle_stiffness: 100.0,
            repulsion_strength: 2000.0,
            repulsion_min_dist: 5.0,
            damping: 0.9,
            max_force: 500.0,
        }
    }
}

/// Force accumulator for a single atom.
#[derive(Debug, Clone, Copy, Default)]
struct ForceAccum {
    force: Vec3,
}

/// Physics solver that relaxes molecules into stable configurations.
pub struct PhysicsSolver {
    config: PhysicsConfig,
    /// Velocity for each atom (indexed by atom index).
    velocities: Vec<Vec3>,
}

impl PhysicsSolver {
    pub fn new(config: PhysicsConfig) -> Self {
        Self {
            config,
            velocities: Vec::new(),
        }
    }

    /// Ensure velocity array matches atom count.
    fn ensure_velocities(&mut self, atom_count: usize) {
        if self.velocities.len() < atom_count {
            self.velocities.resize(atom_count, Vec3::ZERO);
        }
    }

    /// Step the physics simulation forward by dt seconds.
    pub fn step(
        &mut self,
        molecule: &mut MoleculeState3D,
        registry: &ElementRegistry,
        dt: f32,
    ) {
        let atom_count = molecule.atoms.len();
        if atom_count == 0 {
            return;
        }

        self.ensure_velocities(atom_count);

        // Accumulate forces
        let mut forces = vec![ForceAccum::default(); atom_count];

        // 1. Bond spring forces
        self.compute_bond_forces(molecule, registry, &mut forces);

        // 2. Angle (VSEPR) forces
        self.compute_angle_forces(molecule, registry, &mut forces);

        // 3. Repulsion forces (prevent overlap)
        self.compute_repulsion_forces(molecule, registry, &mut forces);

        // Apply forces and integrate
        for (i, atom) in molecule.atoms.iter_mut().enumerate() {
            if atom.dragging {
                // Don't move atoms being dragged
                self.velocities[i] = Vec3::ZERO;
                continue;
            }

            // Clamp force magnitude
            let mut f = forces[i].force;
            let f_mag = f.length();
            if f_mag > self.config.max_force {
                f = f * (self.config.max_force / f_mag);
            }

            // Simple Euler integration with damping
            self.velocities[i] = (self.velocities[i] + f * dt) * self.config.damping;
            atom.position = atom.position + self.velocities[i] * dt;
        }
    }

    /// Compute spring forces to maintain bond lengths.
    fn compute_bond_forces(
        &self,
        molecule: &MoleculeState3D,
        registry: &ElementRegistry,
        forces: &mut [ForceAccum],
    ) {
        for bond in &molecule.bonds {
            let atom_a = match molecule.atoms.get(bond.atom_a) {
                Some(a) => a,
                None => continue,
            };
            let atom_b = match molecule.atoms.get(bond.atom_b) {
                Some(a) => a,
                None => continue,
            };

            // Get ideal bond length from covalent radii
            let radius_a = registry.get(atom_a.element_kind).map_or(20.0, |e| e.radius);
            let radius_b = registry.get(atom_b.element_kind).map_or(20.0, |e| e.radius);
            let ideal_length = radius_a + radius_b;

            // Current bond vector and length
            let delta = atom_b.position - atom_a.position;
            let current_length = delta.length();

            if current_length < 0.001 {
                continue;
            }

            // Spring force: F = k * (current - ideal)
            let displacement = current_length - ideal_length;
            let direction = delta * (1.0 / current_length);
            let force_magnitude = self.config.bond_stiffness * displacement;

            let force = direction * force_magnitude;

            // Apply equal and opposite forces
            forces[bond.atom_a].force = forces[bond.atom_a].force + force;
            forces[bond.atom_b].force = forces[bond.atom_b].force - force;
        }
    }

    /// Compute torque-like forces to maintain VSEPR angles.
    ///
    /// Now considers lone pairs for accurate molecular geometry (e.g., bent H₂O at 104.5°).
    fn compute_angle_forces(
        &self,
        molecule: &MoleculeState3D,
        registry: &ElementRegistry,
        forces: &mut [ForceAccum],
    ) {
        for (_center_idx, center_atom) in molecule.atoms.iter().enumerate() {
            let neighbor_count = center_atom.bonds.len();
            if neighbor_count < 2 {
                continue; // Need at least 2 bonds for angles
            }

            // Calculate lone pairs for proper VSEPR geometry
            let valence = registry
                .get(center_atom.element_kind)
                .map_or(4, |e| e.valence_electrons);
            let lone_pairs = calc_lone_pairs(valence, neighbor_count as u32);

            // Use geometry with lone pairs - this gives correct angles for H₂O, NH₃, etc.
            let geometry = VSEPRGeometry::from_domains(neighbor_count as u8, lone_pairs);
            let ideal_angle_rad = geometry.bond_angle_degrees().to_radians();

            // For each pair of neighbors, compute angle correction force
            for i in 0..neighbor_count {
                for j in (i + 1)..neighbor_count {
                    let neighbor_a_idx = center_atom.bonds[i];
                    let neighbor_b_idx = center_atom.bonds[j];

                    let neighbor_a = match molecule.atoms.get(neighbor_a_idx) {
                        Some(a) => a,
                        None => continue,
                    };
                    let neighbor_b = match molecule.atoms.get(neighbor_b_idx) {
                        Some(a) => a,
                        None => continue,
                    };

                    // Vectors from center to neighbors
                    let vec_a = neighbor_a.position - center_atom.position;
                    let vec_b = neighbor_b.position - center_atom.position;

                    let len_a = vec_a.length();
                    let len_b = vec_b.length();

                    if len_a < 0.001 || len_b < 0.001 {
                        continue;
                    }

                    let dir_a = vec_a * (1.0 / len_a);
                    let dir_b = vec_b * (1.0 / len_b);

                    // Current angle between bonds
                    let dot = dir_a.dot(dir_b).clamp(-1.0, 1.0);
                    let current_angle = dot.acos();

                    // Angle error
                    let angle_error = current_angle - ideal_angle_rad;

                    if angle_error.abs() < 0.01 {
                        continue; // Close enough
                    }

                    // Apply torque as tangential forces on neighbors
                    // The force should push neighbors apart if angle is too small,
                    // or pull them together if angle is too large.
                    let force_mag = self.config.angle_stiffness * angle_error;

                    // Perpendicular directions in the plane of the angle
                    let cross = dir_a.cross(dir_b);
                    let cross_len = cross.length();
                    if cross_len < 0.001 {
                        continue; // Collinear, can't determine plane
                    }
                    let normal = cross * (1.0 / cross_len);

                    // Tangent directions (perpendicular to bond direction, in angle plane)
                    let tangent_a = normal.cross(dir_a);
                    let tangent_b = dir_b.cross(normal);

                    // Apply forces to increase/decrease angle
                    let force_a = tangent_a * force_mag;
                    let force_b = tangent_b * force_mag;

                    forces[neighbor_a_idx].force = forces[neighbor_a_idx].force + force_a;
                    forces[neighbor_b_idx].force = forces[neighbor_b_idx].force + force_b;
                }
            }
        }
    }

    /// Compute repulsion forces between non-bonded atoms.
    fn compute_repulsion_forces(
        &self,
        molecule: &MoleculeState3D,
        registry: &ElementRegistry,
        forces: &mut [ForceAccum],
    ) {
        let atom_count = molecule.atoms.len();

        for i in 0..atom_count {
            for j in (i + 1)..atom_count {
                // Skip if atoms are bonded
                if molecule.has_bond(i, j) {
                    continue;
                }

                let atom_a = &molecule.atoms[i];
                let atom_b = &molecule.atoms[j];

                let delta = atom_b.position - atom_a.position;
                let dist = delta.length();

                // Minimum separation based on radii
                let radius_a = registry.get(atom_a.element_kind).map_or(20.0, |e| e.radius);
                let radius_b = registry.get(atom_b.element_kind).map_or(20.0, |e| e.radius);
                let min_dist = (radius_a + radius_b) * 0.8; // Allow slight overlap

                if dist >= min_dist * 2.0 {
                    continue; // Far enough apart
                }

                let effective_dist = dist.max(self.config.repulsion_min_dist);

                // Inverse square repulsion: F = k / dist^2
                let force_mag = self.config.repulsion_strength / (effective_dist * effective_dist);

                // Stronger repulsion when atoms overlap
                let overlap_factor = if dist < min_dist {
                    2.0 + (min_dist - dist) / min_dist
                } else {
                    1.0
                };

                let direction = if dist > 0.001 {
                    delta * (1.0 / dist)
                } else {
                    // Random direction if overlapping exactly
                    Vec3::new(1.0, 0.0, 0.0)
                };

                let force = direction * (force_mag * overlap_factor);

                // Push atoms apart
                forces[i].force = forces[i].force - force;
                forces[j].force = forces[j].force + force;
            }
        }
    }

    /// Reset velocities (e.g., after adding atoms).
    pub fn reset(&mut self) {
        self.velocities.clear();
    }

    /// Check if the system has reached equilibrium.
    pub fn is_settled(&self, threshold: f32) -> bool {
        self.velocities.iter().all(|v| v.length() < threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zap_engine::EntityId;

    fn mock_registry() -> ElementRegistry {
        ElementRegistry::load().expect("Failed to load periodic table")
    }

    #[test]
    fn bond_spring_pulls_atoms_together() {
        let mut molecule = MoleculeState3D::new();
        let registry = mock_registry();

        // Two carbon atoms far apart
        molecule.add_atom(EntityId(1), 6, Vec3::new(-50.0, 0.0, 0.0));
        molecule.add_atom(EntityId(2), 6, Vec3::new(50.0, 0.0, 0.0));
        molecule.add_bond(0, 1);

        let mut solver = PhysicsSolver::new(PhysicsConfig::default());

        let initial_dist = (molecule.atoms[1].position - molecule.atoms[0].position).length();

        // Run many steps to allow settling
        for _ in 0..500 {
            solver.step(&mut molecule, &registry, 1.0 / 60.0);
        }

        // Atoms should have moved closer together
        let final_dist = (molecule.atoms[1].position - molecule.atoms[0].position).length();
        let carbon_radius = registry.get(6).unwrap().radius;
        let ideal = carbon_radius * 2.0;

        // Just verify movement toward ideal, not exact convergence
        assert!(
            final_dist < initial_dist,
            "Distance {} should be less than initial {}",
            final_dist,
            initial_dist
        );
        assert!(
            (final_dist - ideal).abs() < 50.0,
            "Distance {} should approach ideal {} (within 50)",
            final_dist,
            ideal
        );
    }

    #[test]
    fn repulsion_prevents_overlap() {
        let mut molecule = MoleculeState3D::new();
        let registry = mock_registry();

        // Two non-bonded atoms very close together
        molecule.add_atom(EntityId(1), 6, Vec3::new(0.0, 0.0, 0.0));
        molecule.add_atom(EntityId(2), 6, Vec3::new(5.0, 0.0, 0.0));
        // No bond!

        let mut solver = PhysicsSolver::new(PhysicsConfig::default());

        let initial_dist = (molecule.atoms[1].position - molecule.atoms[0].position).length();

        // Run many steps
        for _ in 0..500 {
            solver.step(&mut molecule, &registry, 1.0 / 60.0);
        }

        // Atoms should have been pushed apart (more than initial)
        let final_dist = (molecule.atoms[1].position - molecule.atoms[0].position).length();
        assert!(
            final_dist > initial_dist,
            "Final distance {} should be > initial {}",
            final_dist,
            initial_dist
        );
    }
}
