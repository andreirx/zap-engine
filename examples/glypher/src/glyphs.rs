use std::collections::HashMap;
use serde::Deserialize;

/// Top-level baked glyph data as exported by the glyph editor.
#[derive(Debug, Deserialize)]
pub struct BakedGlyphs {
    pub meta: GlyphMeta,
    pub glyphs: HashMap<String, GlyphDef>,
}

/// Metadata about the glyph set.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphMeta {
    pub high_exit_letters: Vec<String>,
    pub widths: HashMap<String, u8>,
}

/// Definition of a single glyph (character).
#[derive(Debug, Deserialize)]
pub struct GlyphDef {
    /// Exit type: "Baseline" or "High" (lowercase only, absent for uppercase/digits).
    pub exit: Option<String>,
    /// Character width class: 0 (narrow), 1 (standard), 2 (wide).
    pub width: u8,
    /// Variant name → list of strokes. Each stroke is a list of [x, y] points.
    /// Lowercase: "Baseline" and "High" variants.
    /// Uppercase/digits: single "Default" variant.
    pub variants: HashMap<String, Vec<Vec<[f32; 2]>>>,
}

impl BakedGlyphs {
    /// Parse baked glyph JSON (the format exported by the glyph editor's "Download Baked" button).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get the strokes for a character with the given entry variant.
    /// Returns None if the character or variant is not defined.
    pub fn get_strokes(&self, ch: char, variant: &str) -> Option<&Vec<Vec<[f32; 2]>>> {
        let key = ch.to_string();
        let glyph = self.glyphs.get(&key)?;
        glyph.variants.get(variant)
    }

    /// Get the exit type for a character ("Baseline" or "High").
    /// Defaults to "Baseline" if not specified (uppercase/digits).
    pub fn exit_type(&self, ch: char) -> &str {
        let key = ch.to_string();
        self.glyphs
            .get(&key)
            .and_then(|g| g.exit.as_deref())
            .unwrap_or("Baseline")
    }

    /// Get the width class for a character (0, 1, or 2).
    /// Defaults to 1 (standard) if not in the widths table.
    pub fn width(&self, ch: char) -> u8 {
        let key = ch.to_string();
        self.glyphs
            .get(&key)
            .map(|g| g.width)
            .unwrap_or(1)
    }

    /// Check if this glyph set has data for a given character.
    pub fn has_char(&self, ch: char) -> bool {
        self.glyphs.contains_key(&ch.to_string())
    }

    /// Get the appropriate variant name for a character given the previous letter's exit type.
    /// - Lowercase: "Baseline" or "High" based on prev_exit
    /// - Uppercase/digits: always "Default"
    /// - After space or at start: "Baseline" (for lowercase) or "Default"
    pub fn variant_for(&self, ch: char, prev_exit: &str) -> &'static str {
        if ch.is_ascii_lowercase() {
            if prev_exit == "High" { "High" } else { "Baseline" }
        } else {
            "Default"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_JSON: &str = r#"{
        "meta": {
            "highExitLetters": ["b", "o", "v", "w"],
            "widths": { "i": 0, "m": 2, "w": 2 }
        },
        "glyphs": {
            "a": {
                "exit": "Baseline",
                "width": 1,
                "variants": {
                    "Baseline": [[[0.4, 0.4], [0.6, 0.75]]],
                    "High": [[[0.4, 0.3], [0.6, 0.75]]]
                }
            },
            "o": {
                "exit": "High",
                "width": 1,
                "variants": {
                    "Baseline": [[[0.5, 0.4], [0.5, 0.7]]],
                    "High": [[[0.5, 0.4], [0.5, 0.7]]]
                }
            },
            "A": {
                "width": 1,
                "variants": {
                    "Default": [[[0.3, 0.75], [0.5, 0.15], [0.7, 0.75]]]
                }
            }
        }
    }"#;

    #[test]
    fn parse_baked_glyphs() {
        let glyphs = BakedGlyphs::from_json(TEST_JSON).unwrap();
        assert_eq!(glyphs.meta.high_exit_letters, vec!["b", "o", "v", "w"]);
        assert_eq!(glyphs.glyphs.len(), 3);
    }

    #[test]
    fn get_strokes_lowercase() {
        let glyphs = BakedGlyphs::from_json(TEST_JSON).unwrap();
        let strokes = glyphs.get_strokes('a', "Baseline").unwrap();
        assert_eq!(strokes.len(), 1);
        assert_eq!(strokes[0].len(), 2);
    }

    #[test]
    fn get_strokes_uppercase() {
        let glyphs = BakedGlyphs::from_json(TEST_JSON).unwrap();
        let strokes = glyphs.get_strokes('A', "Default").unwrap();
        assert_eq!(strokes.len(), 1);
        assert!(glyphs.get_strokes('A', "Baseline").is_none());
    }

    #[test]
    fn exit_type() {
        let glyphs = BakedGlyphs::from_json(TEST_JSON).unwrap();
        assert_eq!(glyphs.exit_type('a'), "Baseline");
        assert_eq!(glyphs.exit_type('o'), "High");
        assert_eq!(glyphs.exit_type('A'), "Baseline"); // default for uppercase
    }

    #[test]
    fn variant_selection() {
        let glyphs = BakedGlyphs::from_json(TEST_JSON).unwrap();
        assert_eq!(glyphs.variant_for('a', "Baseline"), "Baseline");
        assert_eq!(glyphs.variant_for('a', "High"), "High");
        assert_eq!(glyphs.variant_for('A', "High"), "Default");
        assert_eq!(glyphs.variant_for('A', "Baseline"), "Default");
    }

    #[test]
    fn width_lookup() {
        let glyphs = BakedGlyphs::from_json(TEST_JSON).unwrap();
        assert_eq!(glyphs.width('a'), 1);
        assert_eq!(glyphs.width('A'), 1);
        assert_eq!(glyphs.width('z'), 1); // default
    }
}
