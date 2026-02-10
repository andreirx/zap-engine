//! Periodic table data structures and registry.
//!
//! Loads element data from embedded JSON (Periodic-Table-JSON, CC-BY-A license).
//! Covalent radii from Cordero et al. (2008) "Covalent radii revisited"
//! Dalton Trans., 2008, 2832-2838. DOI: 10.1039/b801115j

use serde::Deserialize;
use std::collections::HashMap;
use zap_engine::components::mesh::SDFColor;

/// Embed the periodic table JSON at compile time.
const PERIODIC_TABLE_JSON: &str = include_str!("../data/periodic-table.json");

/// Covalent radii in picometers (pm) from Cordero et al. 2008.
/// Index by atomic number (element 0 is placeholder).
/// These are single-bond covalent radii.
const COVALENT_RADII_PM: [u16; 119] = [
    0,    // 0: placeholder
    31,   // 1: H
    28,   // 2: He
    128,  // 3: Li
    96,   // 4: Be
    84,   // 5: B
    76,   // 6: C (sp3)
    71,   // 7: N
    66,   // 8: O
    57,   // 9: F
    58,   // 10: Ne
    166,  // 11: Na
    141,  // 12: Mg
    121,  // 13: Al
    111,  // 14: Si
    107,  // 15: P
    105,  // 16: S
    102,  // 17: Cl
    106,  // 18: Ar
    203,  // 19: K
    176,  // 20: Ca
    170,  // 21: Sc
    160,  // 22: Ti
    153,  // 23: V
    139,  // 24: Cr
    139,  // 25: Mn (low spin)
    132,  // 26: Fe (low spin)
    126,  // 27: Co (low spin)
    124,  // 28: Ni
    132,  // 29: Cu
    122,  // 30: Zn
    122,  // 31: Ga
    120,  // 32: Ge
    119,  // 33: As
    120,  // 34: Se
    120,  // 35: Br
    116,  // 36: Kr
    220,  // 37: Rb
    195,  // 38: Sr
    190,  // 39: Y
    175,  // 40: Zr
    164,  // 41: Nb
    154,  // 42: Mo
    147,  // 43: Tc
    146,  // 44: Ru
    142,  // 45: Rh
    139,  // 46: Pd
    145,  // 47: Ag
    144,  // 48: Cd
    142,  // 49: In
    139,  // 50: Sn
    139,  // 51: Sb
    138,  // 52: Te
    139,  // 53: I
    140,  // 54: Xe
    244,  // 55: Cs
    215,  // 56: Ba
    207,  // 57: La
    204,  // 58: Ce
    203,  // 59: Pr
    201,  // 60: Nd
    199,  // 61: Pm
    198,  // 62: Sm
    198,  // 63: Eu
    196,  // 64: Gd
    194,  // 65: Tb
    192,  // 66: Dy
    192,  // 67: Ho
    189,  // 68: Er
    190,  // 69: Tm
    187,  // 70: Yb
    187,  // 71: Lu
    175,  // 72: Hf
    170,  // 73: Ta
    162,  // 74: W
    151,  // 75: Re
    144,  // 76: Os
    141,  // 77: Ir
    136,  // 78: Pt
    136,  // 79: Au
    132,  // 80: Hg
    145,  // 81: Tl
    146,  // 82: Pb
    148,  // 83: Bi
    140,  // 84: Po
    150,  // 85: At
    150,  // 86: Rn
    260,  // 87: Fr
    221,  // 88: Ra
    215,  // 89: Ac
    206,  // 90: Th
    200,  // 91: Pa
    196,  // 92: U
    190,  // 93: Np
    187,  // 94: Pu
    180,  // 95: Am
    169,  // 96: Cm
    168,  // 97: Bk  (estimated)
    168,  // 98: Cf  (estimated)
    165,  // 99: Es  (estimated)
    167,  // 100: Fm (estimated)
    173,  // 101: Md (estimated)
    176,  // 102: No (estimated)
    161,  // 103: Lr (estimated)
    157,  // 104: Rf (estimated)
    149,  // 105: Db (estimated)
    143,  // 106: Sg (estimated)
    141,  // 107: Bh (estimated)
    134,  // 108: Hs (estimated)
    129,  // 109: Mt (estimated)
    128,  // 110: Ds (estimated)
    121,  // 111: Rg (estimated)
    122,  // 112: Cn (estimated)
    136,  // 113: Nh (estimated)
    143,  // 114: Fl (estimated)
    162,  // 115: Mc (estimated)
    175,  // 116: Lv (estimated)
    165,  // 117: Ts (estimated)
    157,  // 118: Og (estimated)
];

/// Scale factor to convert picometers to screen units.
/// Carbon (76 pm) becomes ~23 screen units, which renders nicely.
const PM_TO_SCREEN: f32 = 0.30;

/// Raw JSON element structure from Periodic-Table-JSON.
#[derive(Debug, Deserialize)]
pub struct RawElement {
    pub number: u32,
    pub symbol: String,
    pub name: String,
    pub atomic_mass: f64,
    pub category: String,
    pub period: u32,
    pub group: Option<u32>,
    pub shells: Vec<u32>,
    #[serde(rename = "cpk-hex")]
    pub cpk_hex: Option<String>,
    pub xpos: u32,
    pub ypos: u32,
}

/// Root structure for the JSON file.
#[derive(Debug, Deserialize)]
pub struct PeriodicTableJSON {
    pub elements: Vec<RawElement>,
}

/// Element category for color coding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementCategory {
    AlkaliMetal,
    AlkalineEarthMetal,
    TransitionMetal,
    PostTransitionMetal,
    Metalloid,
    NonMetal,
    Halogen,
    NobleGas,
    Lanthanide,
    Actinide,
    Unknown,
}

impl ElementCategory {
    /// Parse category string from JSON.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "alkali metal" => Self::AlkaliMetal,
            "alkaline earth metal" => Self::AlkalineEarthMetal,
            "transition metal" => Self::TransitionMetal,
            "post-transition metal" => Self::PostTransitionMetal,
            "metalloid" => Self::Metalloid,
            "diatomic nonmetal" | "polyatomic nonmetal" => Self::NonMetal,
            "halogen" => Self::Halogen,
            "noble gas" => Self::NobleGas,
            "lanthanide" => Self::Lanthanide,
            "actinide" => Self::Actinide,
            _ => Self::Unknown,
        }
    }

    /// Get a UI-friendly color for this category.
    pub fn ui_color(&self) -> &'static str {
        match self {
            Self::AlkaliMetal => "#ff6b6b",
            Self::AlkalineEarthMetal => "#feca57",
            Self::TransitionMetal => "#48dbfb",
            Self::PostTransitionMetal => "#1dd1a1",
            Self::Metalloid => "#5f27cd",
            Self::NonMetal => "#00d2d3",
            Self::Halogen => "#ff9f43",
            Self::NobleGas => "#ff9ff3",
            Self::Lanthanide => "#54a0ff",
            Self::Actinide => "#c8d6e5",
            Self::Unknown => "#576574",
        }
    }
}

/// Processed element data for runtime use.
#[derive(Debug, Clone)]
pub struct ElementData {
    pub atomic_number: u32,
    pub symbol: String,
    pub name: String,
    pub atomic_mass: f64,
    pub category: ElementCategory,
    pub category_name: String,
    pub period: u32,
    pub group: Option<u32>,
    pub shells: Vec<u32>,
    pub color: SDFColor,
    pub xpos: u32,
    pub ypos: u32,
    // Computed fields
    pub valence_electrons: u32,
    pub max_bonds: u8,
    /// Covalent radius in picometers (pm) - actual scientific value.
    pub covalent_radius_pm: u16,
    /// Visual radius in screen units (covalent_radius_pm * PM_TO_SCREEN).
    pub radius: f32,
}

/// Parse CPK hex color to SDFColor.
fn parse_cpk_color(hex: Option<&str>) -> SDFColor {
    hex.and_then(|h| {
        if h.len() < 6 {
            return None;
        }
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some(SDFColor::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0))
    })
    .unwrap_or(SDFColor::new(0.7, 0.7, 0.7))
}

/// Calculate valence electrons from shells array.
fn calc_valence_electrons(shells: &[u32]) -> u32 {
    shells.last().copied().unwrap_or(0)
}

/// Calculate maximum bonds based on period and valence.
/// Uses simplified octet rule: capacity = 2 (period 1) or 8 (period 2+).
fn calc_max_bonds(period: u32, valence: u32) -> u8 {
    let capacity: u32 = if period == 1 { 2 } else { 8 };
    let bonds = capacity.saturating_sub(valence.min(capacity));
    // Cap at 4 bonds for practical simulation
    bonds.min(4) as u8
}

/// Get covalent radius in picometers from atomic number.
/// Uses actual covalent radii from Cordero et al. 2008.
fn get_covalent_radius_pm(atomic_number: u32) -> u16 {
    if (atomic_number as usize) < COVALENT_RADII_PM.len() {
        COVALENT_RADII_PM[atomic_number as usize]
    } else {
        150 // Default fallback for unknown elements
    }
}

/// Element registry with O(1) lookup by atomic number.
pub struct ElementRegistry {
    elements: HashMap<u32, ElementData>,
    by_symbol: HashMap<String, u32>,
}

impl ElementRegistry {
    /// Load the registry from embedded JSON.
    pub fn load() -> Result<Self, serde_json::Error> {
        Self::from_json(PERIODIC_TABLE_JSON)
    }

    /// Parse registry from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let raw: PeriodicTableJSON = serde_json::from_str(json)?;
        let mut elements = HashMap::new();
        let mut by_symbol = HashMap::new();

        for raw_elem in raw.elements {
            let valence = calc_valence_electrons(&raw_elem.shells);
            let max_bonds = calc_max_bonds(raw_elem.period, valence);
            let covalent_pm = get_covalent_radius_pm(raw_elem.number);

            let data = ElementData {
                atomic_number: raw_elem.number,
                symbol: raw_elem.symbol.clone(),
                name: raw_elem.name,
                atomic_mass: raw_elem.atomic_mass,
                category: ElementCategory::from_str(&raw_elem.category),
                category_name: raw_elem.category,
                period: raw_elem.period,
                group: raw_elem.group,
                shells: raw_elem.shells,
                color: parse_cpk_color(raw_elem.cpk_hex.as_deref()),
                xpos: raw_elem.xpos,
                ypos: raw_elem.ypos,
                valence_electrons: valence,
                max_bonds,
                covalent_radius_pm: covalent_pm,
                radius: covalent_pm as f32 * PM_TO_SCREEN,
            };

            by_symbol.insert(raw_elem.symbol, raw_elem.number);
            elements.insert(raw_elem.number, data);
        }

        Ok(Self { elements, by_symbol })
    }

    /// Get element by atomic number.
    pub fn get(&self, atomic_number: u32) -> Option<&ElementData> {
        self.elements.get(&atomic_number)
    }

    /// Get element by symbol.
    pub fn get_by_symbol(&self, symbol: &str) -> Option<&ElementData> {
        self.by_symbol.get(symbol).and_then(|n| self.elements.get(n))
    }

    /// Iterate over all elements.
    pub fn iter(&self) -> impl Iterator<Item = &ElementData> {
        self.elements.values()
    }

    /// Get number of elements.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_registry() {
        let registry = ElementRegistry::load().expect("Failed to load periodic table");
        assert_eq!(registry.len(), 119, "Should have 119 elements");
    }

    #[test]
    fn hydrogen_properties() {
        let registry = ElementRegistry::load().unwrap();
        let h = registry.get(1).expect("Hydrogen should exist");
        assert_eq!(h.symbol, "H");
        assert_eq!(h.name, "Hydrogen");
        assert_eq!(h.shells, vec![1]);
        assert_eq!(h.valence_electrons, 1);
        assert_eq!(h.max_bonds, 1);
        assert_eq!(h.period, 1);
    }

    #[test]
    fn carbon_properties() {
        let registry = ElementRegistry::load().unwrap();
        let c = registry.get(6).expect("Carbon should exist");
        assert_eq!(c.symbol, "C");
        assert_eq!(c.shells, vec![2, 4]);
        assert_eq!(c.valence_electrons, 4);
        assert_eq!(c.max_bonds, 4);
    }

    #[test]
    fn oxygen_properties() {
        let registry = ElementRegistry::load().unwrap();
        let o = registry.get(8).expect("Oxygen should exist");
        assert_eq!(o.symbol, "O");
        assert_eq!(o.shells, vec![2, 6]);
        assert_eq!(o.valence_electrons, 6);
        assert_eq!(o.max_bonds, 2);
    }

    #[test]
    fn nitrogen_properties() {
        let registry = ElementRegistry::load().unwrap();
        let n = registry.get(7).expect("Nitrogen should exist");
        assert_eq!(n.symbol, "N");
        assert_eq!(n.valence_electrons, 5);
        assert_eq!(n.max_bonds, 3);
    }

    #[test]
    fn neon_is_noble_gas() {
        let registry = ElementRegistry::load().unwrap();
        let ne = registry.get(10).expect("Neon should exist");
        assert_eq!(ne.symbol, "Ne");
        assert_eq!(ne.category, ElementCategory::NobleGas);
        assert_eq!(ne.valence_electrons, 8);
        assert_eq!(ne.max_bonds, 0); // Noble gas, full octet
    }

    #[test]
    fn lookup_by_symbol() {
        let registry = ElementRegistry::load().unwrap();
        let fe = registry.get_by_symbol("Fe").expect("Iron should exist");
        assert_eq!(fe.atomic_number, 26);
        assert_eq!(fe.name, "Iron");
    }

    #[test]
    fn cpk_color_parsing() {
        let registry = ElementRegistry::load().unwrap();
        let h = registry.get(1).unwrap();
        // Hydrogen is white: ffffff
        assert!((h.color.r - 1.0).abs() < 0.01);
        assert!((h.color.g - 1.0).abs() < 0.01);
        assert!((h.color.b - 1.0).abs() < 0.01);
    }
}
