use zap_engine::api::types::EntityId;
use zap_engine::JointHandle;

/// A bond between two atoms.
#[derive(Debug, Clone, Copy)]
pub struct Bond {
    pub atom_a: EntityId,
    pub atom_b: EntityId,
    pub visual_entity: EntityId,
    pub joint_handle: JointHandle,
    /// The spring rest length (minimum = sum of atom radii to prevent overlap).
    #[allow(dead_code)]
    pub rest_length: f32,
}

/// Tracks atoms and bonds in the molecule.
pub struct MoleculeState {
    /// (entity_id, element_kind, current_bond_count)
    pub atoms: Vec<(EntityId, u32, u8)>,
    pub bonds: Vec<Bond>,
}

impl MoleculeState {
    pub fn new() -> Self {
        Self {
            atoms: Vec::new(),
            bonds: Vec::new(),
        }
    }

    pub fn add_atom(&mut self, id: EntityId, element_kind: u32) {
        self.atoms.push((id, element_kind, 0));
    }

    pub fn bond_count(&self, id: EntityId) -> u8 {
        self.atoms.iter()
            .find(|(atom_id, _, _)| *atom_id == id)
            .map(|(_, _, count)| *count)
            .unwrap_or(0)
    }

    pub fn element_kind(&self, id: EntityId) -> Option<u32> {
        self.atoms.iter()
            .find(|(atom_id, _, _)| *atom_id == id)
            .map(|(_, kind, _)| *kind)
    }

    pub fn can_bond(&self, id: EntityId, max_bonds: u8) -> bool {
        self.bond_count(id) < max_bonds
    }

    pub fn add_bond(&mut self, bond: Bond) {
        // Increment bond counts for both atoms
        for atom in self.atoms.iter_mut() {
            if atom.0 == bond.atom_a || atom.0 == bond.atom_b {
                atom.2 += 1;
            }
        }
        self.bonds.push(bond);
    }

    pub fn has_bond_between(&self, a: EntityId, b: EntityId) -> bool {
        self.bonds.iter().any(|bond| {
            (bond.atom_a == a && bond.atom_b == b) ||
            (bond.atom_a == b && bond.atom_b == a)
        })
    }
}
