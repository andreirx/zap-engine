//! Legacy element constants for backward compatibility.
//!
//! Element data now comes from `periodic_table::ElementRegistry`.
//! These constants are kept for reference and use in React UI.

/// Common element atomic numbers for quick access.
pub const HYDROGEN: u32 = 1;
pub const HELIUM: u32 = 2;
pub const CARBON: u32 = 6;
pub const NITROGEN: u32 = 7;
pub const OXYGEN: u32 = 8;
pub const SULFUR: u32 = 16;
pub const PHOSPHORUS: u32 = 15;
pub const CHLORINE: u32 = 17;
pub const IRON: u32 = 26;
pub const GOLD: u32 = 79;

/// Common elements for the quick-select UI bar.
pub const COMMON_ELEMENTS: &[u32] = &[
    HYDROGEN,
    CARBON,
    NITROGEN,
    OXYGEN,
    SULFUR,
    PHOSPHORUS,
];
