//! VSEPR (Valence Shell Electron Pair Repulsion) geometry for molecules.
//!
//! Calculates ideal bond angles and positions atoms according to VSEPR theory.
//! Now includes lone pair logic for accurate molecular geometry.

use crate::math3d::Vec3;
use std::f32::consts::TAU;

/// VSEPR molecular geometry based on electron domains (bonds + lone pairs).
///
/// This distinguishes between **electron domain geometry** (arrangement of all
/// electron pairs) and **molecular geometry** (arrangement of atoms only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VSEPRGeometry {
    /// Single atom (no bonds)
    Monatomic,
    /// 1 bond: no defined geometry
    Terminal,
    /// 2 domains, 0 lone pairs: 180° (e.g., BeCl₂, CO₂)
    Linear,
    /// 3 domains, 0 lone pairs: 120° in a plane (e.g., BF₃)
    TrigonalPlanar,
    /// 3 domains, 1 lone pair: ~117° bent (e.g., SO₂, NO₂⁻)
    BentTrigonal,
    /// 4 domains, 0 lone pairs: 109.5° tetrahedron (e.g., CH₄)
    Tetrahedral,
    /// 4 domains, 1 lone pair: ~107° trigonal pyramid (e.g., NH₃)
    TrigonalPyramidal,
    /// 4 domains, 2 lone pairs: ~104.5° bent (e.g., H₂O)
    BentTetrahedral,
    /// 5 domains: 90°/120° (e.g., PCl₅)
    TrigonalBipyramidal,
    /// 5 domains, 1 lone pair: ~90°/120° seesaw (e.g., SF₄)
    Seesaw,
    /// 5 domains, 2 lone pairs: 90° T-shaped (e.g., ClF₃)
    TShaped,
    /// 5 domains, 3 lone pairs: 180° linear (e.g., XeF₂)
    LinearFromTrigBipyr,
    /// 6 domains, 0 lone pairs: 90° (e.g., SF₆)
    Octahedral,
    /// 6 domains, 1 lone pair: 90° square pyramidal (e.g., BrF₅)
    SquarePyramidal,
    /// 6 domains, 2 lone pairs: 90° square planar (e.g., XeF₄)
    SquarePlanar,
}

impl VSEPRGeometry {
    /// Determine geometry from electron domains (bonds + lone pairs).
    ///
    /// This is the scientifically accurate VSEPR lookup.
    pub fn from_domains(bonds: u8, lone_pairs: u8) -> Self {
        let total_domains = bonds + lone_pairs;

        match (total_domains, bonds) {
            (0, _) => Self::Monatomic,
            (1, _) | (_, 1) => Self::Terminal,

            // 2 total domains
            (2, 2) => Self::Linear,
            (2, 1) => Self::Terminal, // 1 bond + 1 lone pair (rare)

            // 3 total domains
            (3, 3) => Self::TrigonalPlanar,
            (3, 2) => Self::BentTrigonal,       // e.g., SO₂
            (3, 1) => Self::Terminal,

            // 4 total domains (most common!)
            (4, 4) => Self::Tetrahedral,        // e.g., CH₄
            (4, 3) => Self::TrigonalPyramidal,  // e.g., NH₃
            (4, 2) => Self::BentTetrahedral,    // e.g., H₂O ← This fixes water!
            (4, 1) => Self::Terminal,

            // 5 total domains
            (5, 5) => Self::TrigonalBipyramidal,
            (5, 4) => Self::Seesaw,
            (5, 3) => Self::TShaped,
            (5, 2) => Self::LinearFromTrigBipyr,
            (5, _) => Self::Terminal,

            // 6 total domains
            (6, 6) => Self::Octahedral,
            (6, 5) => Self::SquarePyramidal,
            (6, 4) => Self::SquarePlanar,
            (6, _) => Self::Terminal,

            // Fallback for unusual cases
            _ => Self::from_bond_count(bonds),
        }
    }

    /// Determine geometry from number of bonds only (legacy, less accurate).
    /// Use `from_domains()` for scientifically accurate geometry.
    pub fn from_bond_count(count: u8) -> Self {
        match count {
            0 => Self::Monatomic,
            1 => Self::Terminal,
            2 => Self::Linear,
            3 => Self::TrigonalPlanar,
            4 => Self::Tetrahedral,
            5 => Self::TrigonalBipyramidal,
            _ => Self::Octahedral,
        }
    }

    /// Get ideal bond angle in degrees.
    pub fn bond_angle_degrees(&self) -> f32 {
        match self {
            Self::Monatomic | Self::Terminal => 0.0,
            Self::Linear | Self::LinearFromTrigBipyr => 180.0,
            Self::TrigonalPlanar => 120.0,
            Self::BentTrigonal => 117.0, // Slightly compressed by lone pair
            Self::Tetrahedral => 109.47, // arccos(-1/3)
            Self::TrigonalPyramidal => 107.0, // Compressed by lone pair (NH₃ is 107°)
            Self::BentTetrahedral => 104.5, // H₂O actual bond angle
            Self::TrigonalBipyramidal => 120.0, // equatorial angle
            Self::Seesaw => 117.0, // approximately
            Self::TShaped => 90.0,
            Self::Octahedral => 90.0,
            Self::SquarePyramidal => 90.0,
            Self::SquarePlanar => 90.0,
        }
    }

    /// Get display name for UI.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Monatomic => "Monatomic",
            Self::Terminal => "Terminal",
            Self::Linear | Self::LinearFromTrigBipyr => "Linear",
            Self::TrigonalPlanar => "Trigonal Planar",
            Self::BentTrigonal => "Bent",
            Self::Tetrahedral => "Tetrahedral",
            Self::TrigonalPyramidal => "Trigonal Pyramidal",
            Self::BentTetrahedral => "Bent",
            Self::TrigonalBipyramidal => "Trigonal Bipyramidal",
            Self::Seesaw => "Seesaw",
            Self::TShaped => "T-Shaped",
            Self::Octahedral => "Octahedral",
            Self::SquarePyramidal => "Square Pyramidal",
            Self::SquarePlanar => "Square Planar",
        }
    }

    /// Get unit vectors pointing in ideal BOND directions from central atom.
    ///
    /// These are the positions where actual atoms go, NOT including lone pair positions.
    /// The directions are derived from the underlying electron domain geometry.
    pub fn bond_directions(&self) -> Vec<Vec3> {
        match self {
            Self::Monatomic => vec![],

            Self::Terminal => {
                vec![Vec3::X]
            }

            Self::Linear | Self::LinearFromTrigBipyr => {
                // 180 degrees apart along X axis
                vec![Vec3::X, Vec3::new(-1.0, 0.0, 0.0)]
            }

            Self::TrigonalPlanar => {
                // 120 degrees apart in XY plane
                let angle = TAU / 3.0;
                vec![
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(angle.cos(), angle.sin(), 0.0),
                    Vec3::new((2.0 * angle).cos(), (2.0 * angle).sin(), 0.0),
                ]
            }

            Self::BentTrigonal => {
                // 2 bonds from trigonal planar base (~117°)
                // Lone pair takes one position, we return 2 bond positions
                let half_angle = (117.0_f32 / 2.0).to_radians();
                vec![
                    Vec3::new(half_angle.cos(), half_angle.sin(), 0.0),
                    Vec3::new(half_angle.cos(), -half_angle.sin(), 0.0),
                ]
            }

            Self::Tetrahedral => {
                // Vertices of tetrahedron on unit sphere
                let cos_tet = -1.0 / 3.0_f32; // cos(109.47)
                let sin_tet = (1.0 - cos_tet * cos_tet).sqrt();
                let phi = TAU / 3.0;

                vec![
                    Vec3::new(0.0, 1.0, 0.0), // Top
                    Vec3::new(sin_tet, cos_tet, 0.0),
                    Vec3::new(sin_tet * phi.cos(), cos_tet, sin_tet * phi.sin()),
                    Vec3::new(
                        sin_tet * (2.0 * phi).cos(),
                        cos_tet,
                        sin_tet * (2.0 * phi).sin(),
                    ),
                ]
            }

            Self::TrigonalPyramidal => {
                // 3 bonds from tetrahedral base (~107° between bonds)
                // One lone pair takes the "top" position
                // Bonds are on a cone around -Y axis
                //
                // Math: For bonds on a cone with half-angle β from -Y axis,
                // the angle θ between adjacent bonds satisfies:
                // cos(θ) = -0.5*sin²(β) + cos²(β) = 1.5*cos²(β) - 0.5
                // Solving for β when θ = 107°:
                let target_angle = 107.0_f32.to_radians();
                let cos_target = target_angle.cos();
                let cos_beta_sq = (cos_target + 0.5) / 1.5;
                let cos_beta = cos_beta_sq.sqrt();
                let sin_beta = (1.0 - cos_beta_sq).sqrt();
                let phi = TAU / 3.0;

                vec![
                    Vec3::new(sin_beta, -cos_beta, 0.0),
                    Vec3::new(sin_beta * phi.cos(), -cos_beta, sin_beta * phi.sin()),
                    Vec3::new(sin_beta * (2.0 * phi).cos(), -cos_beta, sin_beta * (2.0 * phi).sin()),
                ]
            }

            Self::BentTetrahedral => {
                // 2 bonds from tetrahedral base (~104.5°) - WATER geometry
                // Two lone pairs take positions, we return 2 bond positions
                let half_angle = (104.5_f32 / 2.0).to_radians();
                vec![
                    Vec3::new(half_angle.sin(), -half_angle.cos(), 0.0),
                    Vec3::new(-half_angle.sin(), -half_angle.cos(), 0.0),
                ]
            }

            Self::TrigonalBipyramidal => {
                // 3 equatorial at 120 degrees + 2 axial
                let angle = TAU / 3.0;
                vec![
                    Vec3::new(0.0, 1.0, 0.0),  // Axial top
                    Vec3::new(0.0, -1.0, 0.0), // Axial bottom
                    Vec3::new(1.0, 0.0, 0.0),  // Equatorial
                    Vec3::new(angle.cos(), 0.0, angle.sin()),
                    Vec3::new((2.0 * angle).cos(), 0.0, (2.0 * angle).sin()),
                ]
            }

            Self::Seesaw => {
                // 4 bonds from trigonal bipyramidal, lone pair in equatorial
                let angle = TAU / 3.0;
                vec![
                    Vec3::new(0.0, 1.0, 0.0),  // Axial top
                    Vec3::new(0.0, -1.0, 0.0), // Axial bottom
                    Vec3::new(1.0, 0.0, 0.0),  // Equatorial
                    Vec3::new(angle.cos(), 0.0, angle.sin()),
                ]
            }

            Self::TShaped => {
                // 3 bonds from trigonal bipyramidal, 2 lone pairs in equatorial
                vec![
                    Vec3::new(0.0, 1.0, 0.0),  // Axial top
                    Vec3::new(0.0, -1.0, 0.0), // Axial bottom
                    Vec3::new(1.0, 0.0, 0.0),  // Equatorial
                ]
            }

            Self::Octahedral => {
                // 6 directions along axes
                vec![
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, -1.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 0.0, -1.0),
                ]
            }

            Self::SquarePyramidal => {
                // 5 bonds from octahedral, 1 lone pair
                vec![
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 0.0, -1.0),
                ]
            }

            Self::SquarePlanar => {
                // 4 bonds from octahedral, 2 lone pairs along one axis
                vec![
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(0.0, 0.0, -1.0),
                ]
            }
        }
    }
}

/// Calculate the number of lone pairs on an atom.
///
/// Uses the formula: lone_pairs = (valence_electrons - bonding_electrons) / 2
/// where bonding_electrons = 2 * number_of_bonds (assuming single bonds).
///
/// Note: This is simplified and doesn't handle multiple bonds correctly,
/// but works well for most common molecules.
pub fn calc_lone_pairs(valence_electrons: u32, bond_count: u32) -> u8 {
    // Each bond uses 1 valence electron from this atom
    let electrons_in_bonds = bond_count;
    let remaining = valence_electrons.saturating_sub(electrons_in_bonds);
    // Remaining electrons form lone pairs (2 electrons per pair)
    // But for VSEPR we count the number of lone pair positions
    (remaining / 2) as u8
}

/// Calculate ideal bond length based on element covalent radii.
/// Bond length = sum of covalent radii (already scaled from pm to screen units).
pub fn ideal_bond_length(radius_a: f32, radius_b: f32) -> f32 {
    radius_a + radius_b
}

/// Position neighbor atoms around a central atom according to VSEPR geometry.
///
/// Now takes lone_pairs into account for accurate molecular geometry.
pub fn solve_vsepr_positions_with_lone_pairs(
    central_pos: Vec3,
    bond_length: f32,
    bond_count: usize,
    lone_pairs: u8,
    existing_neighbor_dirs: &[Vec3],
) -> Vec<Vec3> {
    if bond_count == 0 {
        return vec![];
    }

    let geometry = VSEPRGeometry::from_domains(bond_count as u8, lone_pairs);
    let ideal_dirs = geometry.bond_directions();

    if bond_count <= ideal_dirs.len() {
        let dirs = if existing_neighbor_dirs.is_empty() {
            ideal_dirs[..bond_count].to_vec()
        } else {
            assign_directions_to_neighbors(existing_neighbor_dirs, &ideal_dirs, bond_count)
        };

        dirs.iter()
            .map(|d| central_pos + (*d * bond_length))
            .collect()
    } else {
        // More neighbors than ideal directions - distribute on sphere
        distribute_on_sphere(central_pos, bond_length, bond_count)
    }
}

/// Legacy function - use solve_vsepr_positions_with_lone_pairs for accuracy.
pub fn solve_vsepr_positions(
    central_pos: Vec3,
    bond_length: f32,
    neighbor_count: usize,
    existing_neighbor_dirs: &[Vec3],
) -> Vec<Vec3> {
    // Legacy: no lone pairs considered
    solve_vsepr_positions_with_lone_pairs(
        central_pos,
        bond_length,
        neighbor_count,
        0, // No lone pairs
        existing_neighbor_dirs,
    )
}

/// Assign ideal directions to existing neighbor positions using greedy matching.
fn assign_directions_to_neighbors(
    existing: &[Vec3],
    ideal: &[Vec3],
    count: usize,
) -> Vec<Vec3> {
    let mut used = vec![false; ideal.len()];
    let mut result = Vec::with_capacity(count);

    // For each existing direction, find closest unused ideal direction
    for dir in existing.iter().take(count) {
        let mut best_idx = 0;
        let mut best_dot = f32::NEG_INFINITY;

        for (i, ideal_dir) in ideal.iter().enumerate() {
            if !used[i] {
                let dot = dir.dot(*ideal_dir);
                if dot > best_dot {
                    best_dot = dot;
                    best_idx = i;
                }
            }
        }

        used[best_idx] = true;
        result.push(ideal[best_idx]);
    }

    // Fill remaining with unused ideal directions
    for (i, ideal_dir) in ideal.iter().enumerate() {
        if result.len() >= count {
            break;
        }
        if !used[i] {
            result.push(*ideal_dir);
        }
    }

    result
}

/// Distribute points evenly on a sphere using Fibonacci spiral.
fn distribute_on_sphere(center: Vec3, radius: f32, count: usize) -> Vec<Vec3> {
    let golden_ratio = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let angle_increment = TAU / golden_ratio;

    (0..count)
        .map(|i| {
            let t = i as f32 / (count - 1).max(1) as f32;
            let inclination = (1.0 - 2.0 * t).acos();
            let azimuth = angle_increment * i as f32;

            let x = inclination.sin() * azimuth.cos();
            let y = inclination.cos();
            let z = inclination.sin() * azimuth.sin();

            center + Vec3::new(x, y, z) * radius
        })
        .collect()
}

/// Calculate the angle between two bond directions (in degrees).
pub fn bond_angle(dir_a: Vec3, dir_b: Vec3) -> f32 {
    let dot = dir_a.normalize().dot(dir_b.normalize());
    dot.clamp(-1.0, 1.0).acos().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_from_bond_count_legacy() {
        assert_eq!(VSEPRGeometry::from_bond_count(0), VSEPRGeometry::Monatomic);
        assert_eq!(VSEPRGeometry::from_bond_count(2), VSEPRGeometry::Linear);
        assert_eq!(VSEPRGeometry::from_bond_count(4), VSEPRGeometry::Tetrahedral);
    }

    #[test]
    fn geometry_from_domains_water() {
        // Water: 2 bonds + 2 lone pairs = 4 domains
        let geom = VSEPRGeometry::from_domains(2, 2);
        assert_eq!(geom, VSEPRGeometry::BentTetrahedral);
        assert!((geom.bond_angle_degrees() - 104.5).abs() < 0.1);
    }

    #[test]
    fn geometry_from_domains_ammonia() {
        // Ammonia: 3 bonds + 1 lone pair = 4 domains
        let geom = VSEPRGeometry::from_domains(3, 1);
        assert_eq!(geom, VSEPRGeometry::TrigonalPyramidal);
        assert!((geom.bond_angle_degrees() - 107.0).abs() < 0.1);
    }

    #[test]
    fn geometry_from_domains_methane() {
        // Methane: 4 bonds + 0 lone pairs = 4 domains
        let geom = VSEPRGeometry::from_domains(4, 0);
        assert_eq!(geom, VSEPRGeometry::Tetrahedral);
        assert!((geom.bond_angle_degrees() - 109.47).abs() < 0.1);
    }

    #[test]
    fn geometry_from_domains_sulfur_dioxide() {
        // SO2: 2 bonds + 1 lone pair = 3 domains
        let geom = VSEPRGeometry::from_domains(2, 1);
        assert_eq!(geom, VSEPRGeometry::BentTrigonal);
        assert!((geom.bond_angle_degrees() - 117.0).abs() < 0.1);
    }

    #[test]
    fn calc_lone_pairs_oxygen() {
        // Oxygen: 6 valence electrons, 2 bonds
        let lp = calc_lone_pairs(6, 2);
        assert_eq!(lp, 2); // (6-2)/2 = 2
    }

    #[test]
    fn calc_lone_pairs_nitrogen() {
        // Nitrogen: 5 valence electrons, 3 bonds
        let lp = calc_lone_pairs(5, 3);
        assert_eq!(lp, 1); // (5-3)/2 = 1
    }

    #[test]
    fn calc_lone_pairs_carbon() {
        // Carbon: 4 valence electrons, 4 bonds
        let lp = calc_lone_pairs(4, 4);
        assert_eq!(lp, 0); // (4-4)/2 = 0
    }

    #[test]
    fn water_bond_directions() {
        // Water should have 2 bond directions at ~104.5°
        let dirs = VSEPRGeometry::BentTetrahedral.bond_directions();
        assert_eq!(dirs.len(), 2);

        let angle = bond_angle(dirs[0], dirs[1]);
        assert!(
            (angle - 104.5).abs() < 1.0,
            "Water bond angle is {}, expected ~104.5°",
            angle
        );
    }

    #[test]
    fn ammonia_bond_directions() {
        // Ammonia should have 3 bond directions at ~107°
        let dirs = VSEPRGeometry::TrigonalPyramidal.bond_directions();
        assert_eq!(dirs.len(), 3);

        // Check angle between any two bonds
        let angle = bond_angle(dirs[0], dirs[1]);
        assert!(
            (angle - 107.0).abs() < 2.0,
            "Ammonia bond angle is {}, expected ~107°",
            angle
        );
    }

    #[test]
    fn linear_directions() {
        let dirs = VSEPRGeometry::Linear.bond_directions();
        assert_eq!(dirs.len(), 2);
        let angle = bond_angle(dirs[0], dirs[1]);
        assert!((angle - 180.0).abs() < 0.1);
    }

    #[test]
    fn trigonal_planar_angles() {
        let dirs = VSEPRGeometry::TrigonalPlanar.bond_directions();
        assert_eq!(dirs.len(), 3);

        let angle_01 = bond_angle(dirs[0], dirs[1]);
        let angle_12 = bond_angle(dirs[1], dirs[2]);
        let angle_20 = bond_angle(dirs[2], dirs[0]);

        assert!((angle_01 - 120.0).abs() < 0.5);
        assert!((angle_12 - 120.0).abs() < 0.5);
        assert!((angle_20 - 120.0).abs() < 0.5);
    }

    #[test]
    fn tetrahedral_angles() {
        let dirs = VSEPRGeometry::Tetrahedral.bond_directions();
        assert_eq!(dirs.len(), 4);

        for i in 0..4 {
            for j in (i + 1)..4 {
                let angle = bond_angle(dirs[i], dirs[j]);
                assert!(
                    (angle - 109.47).abs() < 1.0,
                    "Angle between {} and {} is {}, expected ~109.47",
                    i,
                    j,
                    angle
                );
            }
        }
    }

    #[test]
    fn octahedral_angles() {
        let dirs = VSEPRGeometry::Octahedral.bond_directions();
        assert_eq!(dirs.len(), 6);

        let angle_xy = bond_angle(dirs[0], dirs[2]);
        let angle_opposite = bond_angle(dirs[0], dirs[1]);

        assert!((angle_xy - 90.0).abs() < 0.1);
        assert!((angle_opposite - 180.0).abs() < 0.1);
    }

    #[test]
    fn solve_vsepr_methane() {
        let center = Vec3::ZERO;
        let bond_length = 50.0;
        let positions = solve_vsepr_positions_with_lone_pairs(center, bond_length, 4, 0, &[]);

        assert_eq!(positions.len(), 4);

        for pos in &positions {
            let dist = pos.distance(center);
            assert!((dist - bond_length).abs() < 0.1);
        }

        for i in 0..4 {
            for j in (i + 1)..4 {
                let dir_i = (positions[i] - center).normalize();
                let dir_j = (positions[j] - center).normalize();
                let angle = bond_angle(dir_i, dir_j);
                assert!(
                    (angle - 109.47).abs() < 1.0,
                    "Angle is {}, expected ~109.47",
                    angle
                );
            }
        }
    }

    #[test]
    fn solve_vsepr_water_with_lone_pairs() {
        // Water: 2 bonds + 2 lone pairs
        let center = Vec3::ZERO;
        let bond_length = 40.0;
        let positions = solve_vsepr_positions_with_lone_pairs(center, bond_length, 2, 2, &[]);

        assert_eq!(positions.len(), 2);

        let dir_a = (positions[0] - center).normalize();
        let dir_b = (positions[1] - center).normalize();
        let angle = bond_angle(dir_a, dir_b);

        // Should be bent at ~104.5° now, NOT 180°!
        assert!(
            (angle - 104.5).abs() < 2.0,
            "Water bond angle is {}, expected ~104.5°",
            angle
        );
    }

    #[test]
    fn solve_vsepr_ammonia_with_lone_pairs() {
        // Ammonia: 3 bonds + 1 lone pair
        let center = Vec3::ZERO;
        let bond_length = 40.0;
        let positions = solve_vsepr_positions_with_lone_pairs(center, bond_length, 3, 1, &[]);

        assert_eq!(positions.len(), 3);

        // Check one angle
        let dir_a = (positions[0] - center).normalize();
        let dir_b = (positions[1] - center).normalize();
        let angle = bond_angle(dir_a, dir_b);

        // Should be ~107° (trigonal pyramidal)
        assert!(
            (angle - 107.0).abs() < 3.0,
            "Ammonia bond angle is {}, expected ~107°",
            angle
        );
    }
}
