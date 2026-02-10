//! Chemistry simulation core.
//!
//! Manages the molecular state and physics simulation.

use zap_engine::EntityId;

use crate::math3d::Vec3;
use crate::molecule3d::MoleculeState3D;
use crate::periodic_table::ElementRegistry;
use crate::physics::{PhysicsConfig, PhysicsSolver};

/// Maximum number of atoms allowed in the simulation.
pub const MAX_ATOMS: usize = 1024;

/// The chemistry simulation state and logic.
pub struct ChemistrySim {
    /// Molecular structure state.
    molecule: MoleculeState3D,
    /// Force-directed physics solver.
    physics: PhysicsSolver,
    /// Element registry for looking up properties.
    registry: ElementRegistry,
    /// Entity ID counter.
    next_entity_id: u32,
    /// Currently selected element for spawning.
    selected_element: u32,
    /// Currently selected atom (for UI display).
    selected_atom: Option<usize>,
}

impl ChemistrySim {
    pub fn new(registry: ElementRegistry) -> Self {
        Self {
            molecule: MoleculeState3D::new(),
            physics: PhysicsSolver::new(PhysicsConfig::default()),
            registry,
            next_entity_id: 1,
            selected_element: 6, // Carbon
            selected_atom: None,
        }
    }

    /// Get a reference to the molecule state.
    pub fn molecule(&self) -> &MoleculeState3D {
        &self.molecule
    }

    /// Get the element registry.
    pub fn registry(&self) -> &ElementRegistry {
        &self.registry
    }

    /// Get the currently selected element.
    pub fn selected_element(&self) -> u32 {
        self.selected_element
    }

    /// Set the currently selected element.
    pub fn set_selected_element(&mut self, atomic_number: u32) {
        if self.registry.get(atomic_number).is_some() {
            self.selected_element = atomic_number;
        }
    }

    /// Get the currently selected atom index.
    pub fn selected_atom(&self) -> Option<usize> {
        self.selected_atom
    }

    /// Set the currently selected atom.
    pub fn set_selected_atom(&mut self, atom_idx: Option<usize>) {
        self.selected_atom = atom_idx;
    }

    /// Generate next entity ID.
    fn next_entity_id(&mut self) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        id
    }

    /// Spawn a new atom at the given 3D position.
    /// Returns the atom index if successful.
    pub fn spawn_atom(&mut self, position: Vec3) -> Option<usize> {
        if self.molecule.atoms.len() >= MAX_ATOMS {
            return None;
        }

        if self.registry.get(self.selected_element).is_none() {
            return None;
        }

        let entity_id = self.next_entity_id();
        let atom_idx = self.molecule.add_atom(entity_id, self.selected_element, position);

        // Auto-select newly spawned atom
        self.selected_atom = Some(atom_idx);

        // Reset physics to allow relaxation
        self.physics.reset();

        Some(atom_idx)
    }

    /// Try to create a bond between two atoms.
    /// Returns true if bond was created.
    pub fn try_bond(&mut self, atom_a: usize, atom_b: usize) -> bool {
        if atom_a == atom_b {
            return false;
        }
        if self.molecule.has_bond(atom_a, atom_b) {
            return false;
        }

        let kind_a = match self.molecule.atoms.get(atom_a) {
            Some(a) => a.element_kind,
            None => return false,
        };
        let kind_b = match self.molecule.atoms.get(atom_b) {
            Some(a) => a.element_kind,
            None => return false,
        };

        let elem_a = match self.registry.get(kind_a) {
            Some(e) => e,
            None => return false,
        };
        let elem_b = match self.registry.get(kind_b) {
            Some(e) => e,
            None => return false,
        };

        // Validate using valence rules
        if !self.molecule.can_bond(atom_a, elem_a.max_bonds) {
            return false;
        }
        if !self.molecule.can_bond(atom_b, elem_b.max_bonds) {
            return false;
        }

        // Create bond
        if self.molecule.add_bond(atom_a, atom_b).is_some() {
            // Recompute VSEPR geometry for both atoms
            self.molecule.recompute_geometry(atom_a, &self.registry);
            self.molecule.recompute_geometry(atom_b, &self.registry);

            // Reset physics to allow relaxation
            self.physics.reset();

            return true;
        }

        false
    }

    /// Clear all atoms and bonds.
    pub fn clear(&mut self) {
        self.molecule.clear();
        self.selected_atom = None;
        self.physics.reset();
    }

    /// Step the physics simulation.
    pub fn update(&mut self, dt: f32) {
        self.physics.step(&mut self.molecule, &self.registry, dt);
    }

    /// Check if the simulation has settled.
    pub fn is_settled(&self) -> bool {
        self.physics.is_settled(0.1)
    }

    /// Get atom count.
    pub fn atom_count(&self) -> usize {
        self.molecule.atoms.len()
    }

    /// Get bond count.
    pub fn bond_count(&self) -> usize {
        self.molecule.bonds.len()
    }

    /// Get molecule centroid for camera targeting.
    pub fn centroid(&self) -> Vec3 {
        self.molecule.centroid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_registry() -> ElementRegistry {
        ElementRegistry::load().expect("Failed to load periodic table")
    }

    #[test]
    fn spawn_atom_increments_count() {
        let registry = mock_registry();
        let mut sim = ChemistrySim::new(registry);

        assert_eq!(sim.atom_count(), 0);

        sim.spawn_atom(Vec3::ZERO);
        assert_eq!(sim.atom_count(), 1);

        sim.spawn_atom(Vec3::new(50.0, 0.0, 0.0));
        assert_eq!(sim.atom_count(), 2);
    }

    #[test]
    fn bond_creation_respects_valence() {
        let registry = mock_registry();
        let mut sim = ChemistrySim::new(registry);

        // Set to hydrogen (max 1 bond)
        sim.set_selected_element(1);

        sim.spawn_atom(Vec3::new(0.0, 0.0, 0.0));
        sim.spawn_atom(Vec3::new(50.0, 0.0, 0.0));
        sim.spawn_atom(Vec3::new(100.0, 0.0, 0.0));

        // First bond should succeed
        assert!(sim.try_bond(0, 1));
        assert_eq!(sim.bond_count(), 1);

        // Second bond from atom 0 should fail (hydrogen can only have 1 bond)
        assert!(!sim.try_bond(0, 2));
        assert_eq!(sim.bond_count(), 1);
    }

    #[test]
    fn clear_resets_state() {
        let registry = mock_registry();
        let mut sim = ChemistrySim::new(registry);

        sim.spawn_atom(Vec3::ZERO);
        sim.spawn_atom(Vec3::new(50.0, 0.0, 0.0));
        sim.try_bond(0, 1);

        assert_eq!(sim.atom_count(), 2);
        assert_eq!(sim.bond_count(), 1);

        sim.clear();

        assert_eq!(sim.atom_count(), 0);
        assert_eq!(sim.bond_count(), 0);
    }
}
