//! Pure chemistry logic - valence rules, bonding validation.
//!
//! No engine or rendering dependencies.

/// Calculate valence electrons from shells array.
/// Valence electrons are the electrons in the outermost shell.
pub fn valence_electrons(shells: &[u32]) -> u32 {
    shells.last().copied().unwrap_or(0)
}

/// Calculate shell capacity based on period.
/// Period 1: capacity = 2 (1s orbital)
/// Period 2+: capacity = 8 (simplified octet rule)
pub fn shell_capacity(period: u32) -> u32 {
    if period == 1 { 2 } else { 8 }
}

/// Calculate maximum bonds based on period and valence.
/// Uses simplified octet rule: max_bonds = capacity - valence
pub fn max_bonds(period: u32, valence: u32) -> u8 {
    let capacity = shell_capacity(period);
    let bonds = capacity.saturating_sub(valence.min(capacity));
    // Cap at 4 bonds for practical simulation (carbon is tetravalent)
    bonds.min(4) as u8
}

/// Check if two atoms can form a bond based on their available valence.
pub fn can_bond(
    _valence_a: u32,
    max_bonds_a: u8,
    current_bonds_a: u8,
    _valence_b: u32,
    max_bonds_b: u8,
    current_bonds_b: u8,
) -> bool {
    // Both atoms must have available bond slots
    let available_a = max_bonds_a.saturating_sub(current_bonds_a);
    let available_b = max_bonds_b.saturating_sub(current_bonds_b);

    available_a > 0 && available_b > 0
}

/// Determine if an element is likely to form bonds (not a noble gas).
pub fn is_reactive(valence: u32, period: u32) -> bool {
    let capacity = shell_capacity(period);
    // Full outer shell = noble gas = not reactive
    valence < capacity
}

/// Calculate electronegativity difference for bond type prediction.
/// Returns None if electronegativity data is unavailable.
pub fn bond_polarity(electronegativity_a: Option<f64>, electronegativity_b: Option<f64>) -> Option<f64> {
    match (electronegativity_a, electronegativity_b) {
        (Some(a), Some(b)) => Some((a - b).abs()),
        _ => None,
    }
}

/// Predict bond type based on electronegativity difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BondType {
    /// Electronegativity difference < 0.5
    Nonpolar,
    /// Electronegativity difference 0.5 - 1.7
    Polar,
    /// Electronegativity difference > 1.7
    Ionic,
}

impl BondType {
    pub fn from_polarity(diff: f64) -> Self {
        if diff < 0.5 {
            Self::Nonpolar
        } else if diff < 1.7 {
            Self::Polar
        } else {
            Self::Ionic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valence_hydrogen() {
        assert_eq!(valence_electrons(&[1]), 1);
    }

    #[test]
    fn valence_carbon() {
        assert_eq!(valence_electrons(&[2, 4]), 4);
    }

    #[test]
    fn valence_oxygen() {
        assert_eq!(valence_electrons(&[2, 6]), 6);
    }

    #[test]
    fn valence_empty() {
        assert_eq!(valence_electrons(&[]), 0);
    }

    #[test]
    fn max_bonds_hydrogen() {
        // H: period 1, valence 1 -> capacity 2 -> max_bonds = 1
        assert_eq!(max_bonds(1, 1), 1);
    }

    #[test]
    fn max_bonds_carbon() {
        // C: period 2, valence 4 -> capacity 8 -> max_bonds = 4
        assert_eq!(max_bonds(2, 4), 4);
    }

    #[test]
    fn max_bonds_oxygen() {
        // O: period 2, valence 6 -> capacity 8 -> max_bonds = 2
        assert_eq!(max_bonds(2, 6), 2);
    }

    #[test]
    fn max_bonds_nitrogen() {
        // N: period 2, valence 5 -> capacity 8 -> max_bonds = 3
        assert_eq!(max_bonds(2, 5), 3);
    }

    #[test]
    fn max_bonds_neon() {
        // Ne: period 2, valence 8 -> capacity 8 -> max_bonds = 0
        assert_eq!(max_bonds(2, 8), 0);
    }

    #[test]
    fn max_bonds_helium() {
        // He: period 1, valence 2 -> capacity 2 -> max_bonds = 0
        assert_eq!(max_bonds(1, 2), 0);
    }

    #[test]
    fn can_bond_h_h() {
        // Two hydrogen atoms can bond
        assert!(can_bond(1, 1, 0, 1, 1, 0));
        // But not if one already has a bond
        assert!(!can_bond(1, 1, 1, 1, 1, 0));
    }

    #[test]
    fn can_bond_h_o() {
        // H and O can bond
        assert!(can_bond(1, 1, 0, 6, 2, 0));
        // H-O-H: first H bonds, O has 1 bond
        assert!(can_bond(1, 1, 0, 6, 2, 1));
        // O with 2 bonds is saturated
        assert!(!can_bond(1, 1, 0, 6, 2, 2));
    }

    #[test]
    fn noble_gas_not_reactive() {
        // Neon: valence 8, period 2 -> full octet
        assert!(!is_reactive(8, 2));
        // Helium: valence 2, period 1 -> full shell
        assert!(!is_reactive(2, 1));
    }

    #[test]
    fn reactive_elements() {
        // Hydrogen: valence 1, period 1 -> not full
        assert!(is_reactive(1, 1));
        // Carbon: valence 4, period 2 -> not full
        assert!(is_reactive(4, 2));
    }

    #[test]
    fn bond_polarity_classification() {
        assert_eq!(BondType::from_polarity(0.0), BondType::Nonpolar);
        assert_eq!(BondType::from_polarity(0.4), BondType::Nonpolar);
        assert_eq!(BondType::from_polarity(0.5), BondType::Polar);
        assert_eq!(BondType::from_polarity(1.0), BondType::Polar);
        assert_eq!(BondType::from_polarity(1.7), BondType::Ionic);
        assert_eq!(BondType::from_polarity(3.0), BondType::Ionic);
    }
}
