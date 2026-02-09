//! Segment colors for electric arcs and particles.
//! UV mapping into the arrows.png texture atlas.

use super::rng::Rng;

/// 13 colors from the arrows texture atlas.
/// Matches the TypeScript SEGMENT_COLORS array in constants.ts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SegmentColor {
    Red = 0,
    Orange,
    Yellow,
    LimeGreen,
    Green,
    GreenCyan,
    Cyan,
    SkyBlue,
    Blue,
    Indigo,
    Magenta,
    Pink,
    White,
}

impl SegmentColor {
    pub const ALL: [SegmentColor; 13] = [
        Self::Red, Self::Orange, Self::Yellow, Self::LimeGreen,
        Self::Green, Self::GreenCyan, Self::Cyan, Self::SkyBlue,
        Self::Blue, Self::Indigo, Self::Magenta, Self::Pink, Self::White,
    ];

    pub fn random(rng: &mut Rng) -> Self {
        Self::ALL[rng.next_int(13) as usize]
    }
}

/// UV coordinates for first/middle/last segment parts.
#[derive(Debug, Clone, Copy)]
pub struct SegmentUVs {
    pub first_u: (f32, f32),
    pub first_v: f32,
    pub mid_u: (f32, f32),
    pub mid_v: f32,
    pub last_u: (f32, f32),
    pub last_v: f32,
}

impl SegmentColor {
    /// Returns UV coordinates for first/middle/last segment parts.
    pub fn uvs(&self) -> SegmentUVs {
        let tu: f32 = 1.0 / 16.0;
        let (col, row_block) = match self {
            Self::Red =>       (0, 5),
            Self::Orange =>    (1, 5),
            Self::Yellow =>    (2, 5),
            Self::LimeGreen => (3, 5),
            Self::Green =>     (0, 4),
            Self::GreenCyan => (1, 4),
            Self::Cyan =>      (2, 4),
            Self::SkyBlue =>   (3, 4),
            Self::Blue =>      (0, 3),
            Self::Indigo =>    (1, 3),
            Self::Magenta =>   (2, 3),
            Self::Pink =>      (3, 3),
            Self::White =>     (3, 2),
        };
        let u0 = col as f32 * tu;
        let u1 = (col + 1) as f32 * tu;
        let row = row_block as f32;
        SegmentUVs {
            first_u: (u0, u1),
            first_v: row * tu,
            mid_u: (u0, u1),
            mid_v: (row + 0.5) * tu,
            last_u: (u0, u1),
            last_v: (row + 1.0) * tu,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_color_all_has_13_colors() {
        assert_eq!(SegmentColor::ALL.len(), 13);
    }

    #[test]
    fn segment_color_random_is_valid() {
        let mut rng = Rng::new(42);
        for _ in 0..100 {
            let color = SegmentColor::random(&mut rng);
            assert!(SegmentColor::ALL.contains(&color));
        }
    }

    #[test]
    fn segment_uvs_are_within_bounds() {
        for color in SegmentColor::ALL {
            let uvs = color.uvs();
            assert!(uvs.first_u.0 >= 0.0 && uvs.first_u.1 <= 1.0);
            assert!(uvs.first_v >= 0.0 && uvs.first_v <= 1.0);
        }
    }
}
