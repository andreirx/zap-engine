/// Flag color definitions.
///
/// Each flag maps a grid position (col, row) to an RGB color.
/// All pattern functions use grid indices directly (not normalized)
/// so circles stay circular and crosses have uniform thickness.

pub const FLAG_COUNT: usize = 10;

pub const FLAG_NAMES: [&str; FLAG_COUNT] = [
    "France",
    "Germany",
    "Italy",
    "Romania",
    "Ukraine",
    "Japan",
    "Sweden",
    "Switzerland",
    "USA",
    "UK",
];

/// Returns (r, g, b) in [0, 1] for the given flag at grid position.
pub fn flag_color(flag: usize, col: usize, row: usize, cols: usize, rows: usize) -> (f32, f32, f32) {
    match flag {
        0 => france(col, cols),
        1 => germany(row, rows),
        2 => italy(col, cols),
        3 => romania(col, cols),
        4 => ukraine(row, rows),
        5 => japan(col, row, cols, rows),
        6 => sweden(col, row, cols, rows),
        7 => switzerland(col, row, cols, rows),
        8 => usa(col, row, cols, rows),
        9 => uk(col, row, cols, rows),
        _ => (0.5, 0.5, 0.5),
    }
}

// ── Vertical tricolors ──────────────────────────────────────────────

fn france(col: usize, cols: usize) -> (f32, f32, f32) {
    let third = cols / 3;
    if col < third {
        (0.0, 0.15, 0.60)   // blue
    } else if col < third * 2 {
        (1.0, 1.0, 1.0)     // white
    } else {
        (0.90, 0.10, 0.15)  // red
    }
}

fn italy(col: usize, cols: usize) -> (f32, f32, f32) {
    let third = cols / 3;
    if col < third {
        (0.0, 0.55, 0.27)   // green
    } else if col < third * 2 {
        (1.0, 1.0, 1.0)     // white
    } else {
        (0.80, 0.15, 0.15)  // red
    }
}

fn romania(col: usize, cols: usize) -> (f32, f32, f32) {
    let third = cols / 3;
    if col < third {
        (0.0, 0.16, 0.58)   // blue
    } else if col < third * 2 {
        (0.95, 0.80, 0.0)   // yellow
    } else {
        (0.80, 0.12, 0.15)  // red
    }
}

// ── Horizontal stripes ──────────────────────────────────────────────

fn germany(row: usize, rows: usize) -> (f32, f32, f32) {
    let third = rows / 3;
    if row < third {
        (0.05, 0.05, 0.05)  // black
    } else if row < third * 2 {
        (0.85, 0.10, 0.10)  // red
    } else {
        (1.0, 0.80, 0.0)    // gold
    }
}

fn ukraine(row: usize, rows: usize) -> (f32, f32, f32) {
    if row < rows / 2 {
        (0.0, 0.35, 0.75)   // blue
    } else {
        (1.0, 0.85, 0.0)    // yellow
    }
}

// ── Circle (grid-index distance for true circle) ────────────────────

fn japan(col: usize, row: usize, cols: usize, rows: usize) -> (f32, f32, f32) {
    let cx = (cols - 1) as f32 / 2.0;
    let cy = (rows - 1) as f32 / 2.0;
    let dx = col as f32 - cx;
    let dy = row as f32 - cy;
    let dist = (dx * dx + dy * dy).sqrt();
    // Radius ~30% of the shorter dimension (rows)
    let radius = (rows - 1) as f32 * 0.30;
    if dist < radius {
        (0.75, 0.0, 0.10)   // red circle
    } else {
        (1.0, 1.0, 1.0)     // white
    }
}

// ── Cross flags (grid-index thresholds for uniform thickness) ───────

fn sweden(col: usize, row: usize, cols: usize, rows: usize) -> (f32, f32, f32) {
    // Scandinavian cross: vertical bar offset left (~1/3), horizontal at center
    let cross_col = (cols as f32 * 0.36) as usize; // ~col 8 for 24 cols
    let cross_row_center = (rows - 1) as f32 / 2.0;

    let on_v = (col as isize - cross_col as isize).unsigned_abs() <= 1;
    let on_h = (row as f32 - cross_row_center).abs() <= 1.1;

    if on_h || on_v {
        (1.0, 0.80, 0.0)    // yellow cross
    } else {
        (0.0, 0.30, 0.60)   // blue background
    }
}

fn switzerland(col: usize, row: usize, cols: usize, rows: usize) -> (f32, f32, f32) {
    let cx = (cols - 1) as f32 / 2.0;
    let cy = (rows - 1) as f32 / 2.0;
    let dx = (col as f32 - cx).abs();
    let dy = (row as f32 - cy).abs();

    // Horizontal arm: narrow vertically, wide horizontally
    let on_h = dy <= 1.1 && dx <= 4.0;
    // Vertical arm: narrow horizontally, tall vertically
    let on_v = dx <= 1.1 && dy <= 4.0;

    if on_h || on_v {
        (1.0, 1.0, 1.0)     // white cross
    } else {
        (0.80, 0.05, 0.10)  // red background
    }
}

// ── USA (proper alternating stripes + stars in canton) ──────────────

fn usa(col: usize, row: usize, cols: usize, rows: usize) -> (f32, f32, f32) {
    // Canton: top-left ~2/5 width × ~7/13 height
    let canton_cols = (cols * 2 + 4) / 5; // ~10 cols for 24
    let canton_rows = rows / 2;            // top half
    let in_canton = col < canton_cols && row < canton_rows;

    if in_canton {
        // Stars: every 2nd col on every 2nd row → ~25% white (1 star per 3 blue)
        if col % 2 == 0 && row % 2 == 0 {
            (1.0, 1.0, 1.0)     // white star
        } else {
            (0.05, 0.10, 0.40)  // dark blue
        }
    } else if row % 2 == 0 {
        (0.75, 0.10, 0.15)  // red stripe
    } else {
        (1.0, 1.0, 1.0)     // white stripe
    }
}

// ── UK (Union Jack) ─────────────────────────────────────────────────

fn uk(col: usize, row: usize, cols: usize, rows: usize) -> (f32, f32, f32) {
    let cx = (cols - 1) as f32 / 2.0;
    let cy = (rows - 1) as f32 / 2.0;
    let dx = col as f32 - cx;
    let dy = row as f32 - cy;

    // Normalize to [-1, 1] range for diagonal computation
    let nx = dx / cx;
    let ny = dy / cy;

    // Layer 1 (top): St George's cross — red
    let red_cross_v = dx.abs() <= 1.0;
    let red_cross_h = dy.abs() <= 0.8;

    // Layer 2: White border around St George's cross
    let white_cross_v = dx.abs() <= 1.8;
    let white_cross_h = dy.abs() <= 1.5;

    // Layer 3: Red diagonal saltire (St Patrick's)
    let d1 = (nx - ny).abs(); // main diagonal distance
    let d2 = (nx + ny).abs(); // anti-diagonal distance
    let red_diag = d1 < 0.14 || d2 < 0.14;

    // Layer 4: White diagonal saltire (St Andrew's) — wider
    let white_diag = d1 < 0.28 || d2 < 0.28;

    // Compose from top to bottom priority
    if red_cross_v || red_cross_h {
        (0.80, 0.10, 0.15)  // red St George's cross
    } else if white_cross_v || white_cross_h {
        (1.0, 1.0, 1.0)     // white border
    } else if red_diag {
        (0.80, 0.10, 0.15)  // red St Patrick's saltire
    } else if white_diag {
        (1.0, 1.0, 1.0)     // white St Andrew's saltire
    } else {
        (0.0, 0.15, 0.45)   // dark blue background
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_flags_return_valid_colors() {
        for flag in 0..FLAG_COUNT {
            for row in 0..16 {
                for col in 0..24 {
                    let (r, g, b) = flag_color(flag, col, row, 24, 16);
                    assert!(r >= 0.0 && r <= 1.5, "flag {flag} ({col},{row}) r={r}");
                    assert!(g >= 0.0 && g <= 1.5, "flag {flag} ({col},{row}) g={g}");
                    assert!(b >= 0.0 && b <= 1.5, "flag {flag} ({col},{row}) b={b}");
                }
            }
        }
    }

    #[test]
    fn france_has_three_vertical_bands() {
        let (_, _, b0) = flag_color(0, 0, 8, 24, 16);      // left = blue
        let (r1, g1, _) = flag_color(0, 12, 8, 24, 16);    // center = white
        let (r2, _, b2) = flag_color(0, 23, 8, 24, 16);     // right = red
        assert!(b0 > 0.5, "left should be blue");
        assert!(r1 > 0.9 && g1 > 0.9, "center should be white");
        assert!(r2 > 0.7 && b2 < 0.3, "right should be red");
    }

    #[test]
    fn japan_circle_is_round() {
        // Check symmetric points at equal grid distance from center
        // Point 3 cells right of center
        let (r_right, _, _) = flag_color(5, 15, 8, 24, 16);
        // Point 3 cells below center
        let (r_below, _, _) = flag_color(5, 12, 11, 24, 16);
        // Both should be the same color (both inside or both outside)
        assert_eq!(r_right > 0.5, r_below > 0.5, "circle should be symmetric");
    }

    #[test]
    fn usa_stripes_alternate() {
        // Outside the canton, every row should alternate red/white
        for row in 0..16 {
            let (r, _, _) = flag_color(8, 20, row, 24, 16);
            if row % 2 == 0 {
                assert!(r > 0.5, "even row {row} should be red, got r={r}");
            } else {
                assert!(r > 0.9, "odd row {row} should be white, got r={r}");
            }
        }
    }

    #[test]
    fn usa_stars_sparse() {
        // In the canton, ~25% should be white (every 2nd col on every 2nd row)
        let mut white_count = 0;
        let mut total = 0;
        for row in 0..8 {
            for col in 0..10 {
                let (r, g, _) = flag_color(8, col, row, 24, 16);
                total += 1;
                if r > 0.9 && g > 0.9 {
                    white_count += 1;
                }
            }
        }
        let ratio = white_count as f32 / total as f32;
        assert!(ratio > 0.20 && ratio < 0.30, "star ratio should be ~25%, got {ratio:.0}%");
    }

    #[test]
    fn uk_has_red_center_cross() {
        // Dead center should be red (St George's cross)
        let (r, g, _) = flag_color(9, 12, 8, 24, 16);
        assert!(r > 0.5 && g < 0.3, "center should be red");
    }
}
