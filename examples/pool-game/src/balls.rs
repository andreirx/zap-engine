//! Pool ball data: colors, types, and rack positions.

use glam::Vec2;
use zap_engine::components::mesh::SDFColor;

/// Ball type: solid (1-8) or striped (9-15)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BallType {
    Cue,
    Solid,
    Striped,
}

/// Pool ball definition
#[derive(Debug, Clone, Copy)]
pub struct BallDef {
    pub number: u8,
    pub ball_type: BallType,
    pub color: SDFColor,
}

/// All 16 pool balls (cue + 15 numbered)
pub const BALLS: [BallDef; 16] = [
    // Cue ball (0 in array, not numbered on table)
    BallDef { number: 0, ball_type: BallType::Cue, color: SDFColor { r: 1.0, g: 1.0, b: 1.0 } },
    // Solid balls 1-8
    BallDef { number: 1, ball_type: BallType::Solid, color: SDFColor { r: 1.0, g: 0.84, b: 0.0 } },      // Yellow
    BallDef { number: 2, ball_type: BallType::Solid, color: SDFColor { r: 0.0, g: 0.0, b: 0.7 } },       // Blue
    BallDef { number: 3, ball_type: BallType::Solid, color: SDFColor { r: 0.86, g: 0.0, b: 0.0 } },      // Red
    BallDef { number: 4, ball_type: BallType::Solid, color: SDFColor { r: 0.39, g: 0.0, b: 0.55 } },     // Purple
    BallDef { number: 5, ball_type: BallType::Solid, color: SDFColor { r: 1.0, g: 0.39, b: 0.0 } },      // Orange
    BallDef { number: 6, ball_type: BallType::Solid, color: SDFColor { r: 0.0, g: 0.47, b: 0.0 } },      // Green
    BallDef { number: 7, ball_type: BallType::Solid, color: SDFColor { r: 0.51, g: 0.12, b: 0.12 } },    // Maroon
    BallDef { number: 8, ball_type: BallType::Solid, color: SDFColor { r: 0.04, g: 0.04, b: 0.04 } },    // Black
    // Striped balls 9-15 (same colors as 1-7)
    BallDef { number: 9, ball_type: BallType::Striped, color: SDFColor { r: 1.0, g: 0.84, b: 0.0 } },    // Yellow stripe
    BallDef { number: 10, ball_type: BallType::Striped, color: SDFColor { r: 0.0, g: 0.0, b: 0.7 } },    // Blue stripe
    BallDef { number: 11, ball_type: BallType::Striped, color: SDFColor { r: 0.86, g: 0.0, b: 0.0 } },   // Red stripe
    BallDef { number: 12, ball_type: BallType::Striped, color: SDFColor { r: 0.39, g: 0.0, b: 0.55 } },  // Purple stripe
    BallDef { number: 13, ball_type: BallType::Striped, color: SDFColor { r: 1.0, g: 0.39, b: 0.0 } },   // Orange stripe
    BallDef { number: 14, ball_type: BallType::Striped, color: SDFColor { r: 0.0, g: 0.47, b: 0.0 } },   // Green stripe
    BallDef { number: 15, ball_type: BallType::Striped, color: SDFColor { r: 0.51, g: 0.12, b: 0.12 } }, // Maroon stripe
];

/// Standard 8-ball triangle rack layout.
/// Returns positions relative to the rack apex (first ball position).
/// The apex points LEFT toward the cue ball, rows spread RIGHT.
///
/// Standard layout (viewed from above, cue ball on left):
/// ```text
///  1          <- apex (rightmost, row 0)
///  9   2      <- row 1
///  3   8  10  <- row 2
/// 11  4  5  12 <- row 3
///  6 13 14  7 15 <- row 4
/// ```
pub fn rack_positions(apex: Vec2, ball_radius: f32) -> [Vec2; 15] {
    // Gap between balls (tight rack)
    let gap = ball_radius * 2.0 + 1.0;
    let row_offset = gap * 0.866; // sqrt(3)/2 for equilateral triangle

    let mut positions = [Vec2::ZERO; 15];

    // Layout: (ball_number, row, vertical_offset_from_center)
    // Rows spread to the RIGHT (X increases), balls spread vertically (Y)
    let layout: [(u8, usize, f32); 15] = [
        // (ball_number, row, vertical_offset)
        (1, 0, 0.0),
        (9, 1, -0.5), (2, 1, 0.5),
        (3, 2, -1.0), (8, 2, 0.0), (10, 2, 1.0),
        (11, 3, -1.5), (4, 3, -0.5), (5, 3, 0.5), (12, 3, 1.5),
        (6, 4, -2.0), (13, 4, -1.0), (14, 4, 0.0), (7, 4, 1.0), (15, 4, 2.0),
    ];

    for (ball_num, row, v_offset) in layout {
        // Apex points left, rows go right (X increases with row)
        let x = apex.x + (row as f32) * row_offset;
        // Vertical spread centered on apex.y
        let y = apex.y + v_offset * gap;
        positions[(ball_num - 1) as usize] = Vec2::new(x, y);
    }

    positions
}
