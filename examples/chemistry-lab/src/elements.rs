use zap_engine::components::mesh::SDFColor;

/// Data for a chemical element.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct ElementData {
    pub symbol: &'static str,
    pub name: &'static str,
    pub radius: f32,
    pub color: SDFColor,
    pub max_bonds: u8,
}

/// Element index constants (matches custom event kind values)
pub const HYDROGEN: u32 = 1;
pub const OXYGEN: u32 = 2;
pub const CARBON: u32 = 3;
pub const NITROGEN: u32 = 4;

pub fn element_data(kind: u32) -> Option<ElementData> {
    match kind {
        HYDROGEN => Some(ElementData {
            symbol: "H",
            name: "Hydrogen",
            radius: 25.0,
            color: SDFColor { r: 0.9, g: 0.9, b: 0.9 },
            max_bonds: 1,
        }),
        OXYGEN => Some(ElementData {
            symbol: "O",
            name: "Oxygen",
            radius: 35.0,
            color: SDFColor { r: 0.9, g: 0.2, b: 0.2 },
            max_bonds: 2,
        }),
        CARBON => Some(ElementData {
            symbol: "C",
            name: "Carbon",
            radius: 40.0,
            color: SDFColor { r: 0.3, g: 0.3, b: 0.3 },
            max_bonds: 4,
        }),
        NITROGEN => Some(ElementData {
            symbol: "N",
            name: "Nitrogen",
            radius: 32.0,
            color: SDFColor { r: 0.2, g: 0.4, b: 0.9 },
            max_bonds: 3,
        }),
        _ => None,
    }
}
