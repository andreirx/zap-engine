//! 3D molecule state management.
//!
//! Tracks atoms and bonds with 3D positions, recomputes VSEPR geometry on changes.

use crate::math3d::Vec3;
use crate::vsepr::{calc_lone_pairs, ideal_bond_length, solve_vsepr_positions_with_lone_pairs, VSEPRGeometry};
use crate::periodic_table::ElementRegistry;
use zap_engine::EntityId;

/// 3D atom with position and bond information.
#[derive(Debug, Clone)]
pub struct Atom3D {
    /// Engine entity ID for rendering.
    pub entity_id: EntityId,
    /// Element atomic number.
    pub element_kind: u32,
    /// 3D world position.
    pub position: Vec3,
    /// Indices of bonded atoms in MoleculeState3D.atoms.
    pub bonds: Vec<usize>,
    /// Whether this atom is currently being dragged.
    pub dragging: bool,
}

impl Atom3D {
    pub fn new(entity_id: EntityId, element_kind: u32, position: Vec3) -> Self {
        Self {
            entity_id,
            element_kind,
            position,
            bonds: Vec::new(),
            dragging: false,
        }
    }

    /// Get VSEPR geometry considering lone pairs.
    ///
    /// Requires valence electrons from ElementRegistry for accurate geometry.
    pub fn geometry_with_registry(&self, registry: &ElementRegistry) -> VSEPRGeometry {
        let valence = registry
            .get(self.element_kind)
            .map_or(4, |e| e.valence_electrons);
        let bond_count = self.bonds.len() as u32;
        let lone_pairs = calc_lone_pairs(valence, bond_count);
        VSEPRGeometry::from_domains(bond_count as u8, lone_pairs)
    }

    /// Get VSEPR geometry based on current bond count (legacy, less accurate).
    pub fn geometry(&self) -> VSEPRGeometry {
        VSEPRGeometry::from_bond_count(self.bonds.len() as u8)
    }
}

/// Bond between two atoms.
#[derive(Debug, Clone)]
pub struct Bond3D {
    /// Index of first atom.
    pub atom_a: usize,
    /// Index of second atom.
    pub atom_b: usize,
    /// Engine entity ID for bond visual (capsule).
    pub visual_entity: Option<EntityId>,
}

impl Bond3D {
    pub fn new(atom_a: usize, atom_b: usize) -> Self {
        Self {
            atom_a,
            atom_b,
            visual_entity: None,
        }
    }

    /// Check if this bond connects the given atoms (order-independent).
    pub fn connects(&self, a: usize, b: usize) -> bool {
        (self.atom_a == a && self.atom_b == b) || (self.atom_a == b && self.atom_b == a)
    }
}

/// Complete 3D molecule state.
#[derive(Debug, Clone, Default)]
pub struct MoleculeState3D {
    /// All atoms in the molecule.
    pub atoms: Vec<Atom3D>,
    /// All bonds between atoms.
    pub bonds: Vec<Bond3D>,
    /// Next entity ID counter (managed by game).
    next_entity_id: u32,
}

impl MoleculeState3D {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the entity ID counter (called from game).
    pub fn set_next_entity_id(&mut self, id: u32) {
        self.next_entity_id = id;
    }

    /// Get next entity ID and increment counter.
    pub fn next_entity_id(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        id
    }

    /// Add a new atom at the given position.
    pub fn add_atom(&mut self, entity_id: EntityId, element_kind: u32, position: Vec3) -> usize {
        let idx = self.atoms.len();
        self.atoms.push(Atom3D::new(entity_id, element_kind, position));
        idx
    }

    /// Find atom index by entity ID.
    pub fn find_atom(&self, entity_id: EntityId) -> Option<usize> {
        self.atoms.iter().position(|a| a.entity_id == entity_id)
    }

    /// Check if two atoms are already bonded.
    pub fn has_bond(&self, atom_a: usize, atom_b: usize) -> bool {
        self.bonds.iter().any(|b| b.connects(atom_a, atom_b))
    }

    /// Get number of bonds for an atom.
    pub fn bond_count(&self, atom_idx: usize) -> usize {
        self.atoms.get(atom_idx).map_or(0, |a| a.bonds.len())
    }

    /// Check if an atom can form another bond based on max_bonds.
    pub fn can_bond(&self, atom_idx: usize, max_bonds: u8) -> bool {
        self.bond_count(atom_idx) < max_bonds as usize
    }

    /// Create a bond between two atoms.
    pub fn add_bond(&mut self, atom_a: usize, atom_b: usize) -> Option<usize> {
        if atom_a == atom_b {
            return None;
        }
        if self.has_bond(atom_a, atom_b) {
            return None;
        }
        if atom_a >= self.atoms.len() || atom_b >= self.atoms.len() {
            return None;
        }

        let bond_idx = self.bonds.len();
        self.bonds.push(Bond3D::new(atom_a, atom_b));

        // Update atom bond lists
        self.atoms[atom_a].bonds.push(atom_b);
        self.atoms[atom_b].bonds.push(atom_a);

        Some(bond_idx)
    }

    /// Set the visual entity for a bond.
    pub fn set_bond_visual(&mut self, bond_idx: usize, entity_id: EntityId) {
        if let Some(bond) = self.bonds.get_mut(bond_idx) {
            bond.visual_entity = Some(entity_id);
        }
    }

    /// Calculate centroid of all atoms (for camera target).
    pub fn centroid(&self) -> Vec3 {
        if self.atoms.is_empty() {
            return Vec3::ZERO;
        }

        let sum = self.atoms.iter().fold(Vec3::ZERO, |acc, a| acc + a.position);
        sum * (1.0 / self.atoms.len() as f32)
    }

    /// Recompute VSEPR geometry for an atom and its neighbors.
    ///
    /// Now considers lone pairs for accurate molecular geometry (e.g., bent H₂O).
    pub fn recompute_geometry(&mut self, atom_idx: usize, registry: &ElementRegistry) {
        if atom_idx >= self.atoms.len() {
            return;
        }

        // Skip if atom is being dragged
        if self.atoms[atom_idx].dragging {
            return;
        }

        let neighbor_indices: Vec<usize> = self.atoms[atom_idx].bonds.clone();
        if neighbor_indices.is_empty() {
            return;
        }

        let central_pos = self.atoms[atom_idx].position;
        let central_element = self.atoms[atom_idx].element_kind;

        // Get central atom's properties
        let (central_radius, central_valence) = registry
            .get(central_element)
            .map_or((20.0, 4), |e| (e.radius, e.valence_electrons));

        // Calculate lone pairs for the CENTRAL atom (this is what matters for VSEPR!)
        let bond_count = neighbor_indices.len() as u32;
        let lone_pairs = calc_lone_pairs(central_valence, bond_count);

        // Collect existing neighbor directions
        let existing_dirs: Vec<Vec3> = neighbor_indices
            .iter()
            .filter_map(|&i| {
                self.atoms.get(i).map(|a| (a.position - central_pos).normalize())
            })
            .collect();

        // Calculate average bond length based on element radii
        let avg_neighbor_radius = neighbor_indices
            .iter()
            .filter_map(|&i| {
                self.atoms.get(i).and_then(|a| {
                    registry.get(a.element_kind).map(|e| e.radius)
                })
            })
            .sum::<f32>()
            / neighbor_indices.len().max(1) as f32;

        let bond_length = ideal_bond_length(central_radius, avg_neighbor_radius);

        // Solve VSEPR for new positions WITH LONE PAIRS
        let new_positions = solve_vsepr_positions_with_lone_pairs(
            central_pos,
            bond_length,
            neighbor_indices.len(),
            lone_pairs,
            &existing_dirs,
        );

        // Update neighbor positions (only non-dragged atoms)
        for (i, &neighbor_idx) in neighbor_indices.iter().enumerate() {
            if let Some(neighbor) = self.atoms.get_mut(neighbor_idx) {
                if !neighbor.dragging && i < new_positions.len() {
                    neighbor.position = new_positions[i];
                }
            }
        }
    }

    /// Recompute geometry for all atoms with bonds.
    pub fn recompute_all_geometry(&mut self, registry: &ElementRegistry) {
        // Process atoms with most bonds first (they're more "central")
        let mut atom_indices: Vec<usize> = (0..self.atoms.len())
            .filter(|&i| !self.atoms[i].bonds.is_empty())
            .collect();

        atom_indices.sort_by_key(|&i| std::cmp::Reverse(self.atoms[i].bonds.len()));

        for idx in atom_indices {
            self.recompute_geometry(idx, registry);
        }
    }

    /// Clear all atoms and bonds.
    pub fn clear(&mut self) {
        self.atoms.clear();
        self.bonds.clear();
    }

    /// Get all bond visual entities (for cleanup).
    pub fn bond_visuals(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.bonds.iter().filter_map(|b| b.visual_entity)
    }

    /// Hit test: find atom near the given 3D position.
    pub fn hit_test_3d(&self, pos: Vec3, registry: &ElementRegistry) -> Option<usize> {
        for (i, atom) in self.atoms.iter().enumerate() {
            let radius = registry.get(atom.element_kind).map_or(20.0, |e| e.radius);
            if atom.position.distance(pos) < radius * 2.0 {
                return Some(i);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_atom() {
        let mut mol = MoleculeState3D::new();
        let idx = mol.add_atom(EntityId(1), 6, Vec3::ZERO);
        assert_eq!(idx, 0);
        assert_eq!(mol.atoms.len(), 1);
        assert_eq!(mol.atoms[0].element_kind, 6);
    }

    #[test]
    fn add_bond() {
        let mut mol = MoleculeState3D::new();
        mol.add_atom(EntityId(1), 6, Vec3::ZERO);
        mol.add_atom(EntityId(2), 1, Vec3::X * 50.0);

        let bond_idx = mol.add_bond(0, 1);
        assert!(bond_idx.is_some());
        assert_eq!(mol.bonds.len(), 1);
        assert_eq!(mol.atoms[0].bonds, vec![1]);
        assert_eq!(mol.atoms[1].bonds, vec![0]);
    }

    #[test]
    fn no_duplicate_bonds() {
        let mut mol = MoleculeState3D::new();
        mol.add_atom(EntityId(1), 6, Vec3::ZERO);
        mol.add_atom(EntityId(2), 1, Vec3::X * 50.0);

        mol.add_bond(0, 1);
        let dup = mol.add_bond(0, 1);
        assert!(dup.is_none());
        assert_eq!(mol.bonds.len(), 1);
    }

    #[test]
    fn no_self_bonds() {
        let mut mol = MoleculeState3D::new();
        mol.add_atom(EntityId(1), 6, Vec3::ZERO);

        let self_bond = mol.add_bond(0, 0);
        assert!(self_bond.is_none());
    }

    #[test]
    fn centroid_calculation() {
        let mut mol = MoleculeState3D::new();
        mol.add_atom(EntityId(1), 1, Vec3::new(0.0, 0.0, 0.0));
        mol.add_atom(EntityId(2), 1, Vec3::new(100.0, 0.0, 0.0));

        let c = mol.centroid();
        assert!((c.x - 50.0).abs() < 0.01);
        assert!((c.y).abs() < 0.01);
    }

    #[test]
    fn bond_connects() {
        let bond = Bond3D::new(0, 1);
        assert!(bond.connects(0, 1));
        assert!(bond.connects(1, 0));
        assert!(!bond.connects(0, 2));
    }

    #[test]
    fn find_atom_by_entity() {
        let mut mol = MoleculeState3D::new();
        mol.add_atom(EntityId(42), 6, Vec3::ZERO);
        mol.add_atom(EntityId(99), 1, Vec3::X);

        assert_eq!(mol.find_atom(EntityId(42)), Some(0));
        assert_eq!(mol.find_atom(EntityId(99)), Some(1));
        assert_eq!(mol.find_atom(EntityId(100)), None);
    }
}
