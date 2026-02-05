use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Asset manifest describing all atlases and named sprites for a game.
/// Loaded from a JSON file at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    /// List of texture atlases.
    pub atlases: Vec<AtlasDescriptor>,
    /// Named sprite lookup: name → atlas index + cell coordinates.
    #[serde(default)]
    pub sprites: HashMap<String, SpriteDescriptor>,
    /// Optional audio assets.
    #[serde(default)]
    pub sounds: HashMap<String, SoundDescriptor>,
}

/// Describes a single texture atlas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasDescriptor {
    /// Human-readable name (e.g., "base_tiles").
    pub name: String,
    /// Number of columns in the atlas grid.
    pub cols: u32,
    /// Number of rows in the atlas grid.
    pub rows: u32,
    /// Relative path to the PNG file (e.g., "base_tiles.png").
    pub path: String,
}

/// Describes a named sprite within an atlas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteDescriptor {
    /// Index into the atlases array.
    pub atlas: u32,
    /// Column in the atlas grid.
    pub col: u32,
    /// Row in the atlas grid.
    pub row: u32,
    /// Number of cells this sprite spans (default: 1).
    #[serde(default = "default_span")]
    pub span: u32,
}

/// Describes an audio asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundDescriptor {
    /// Relative path to the audio file.
    pub path: String,
    /// Numeric event ID that triggers this sound from Rust.
    #[serde(default)]
    pub event_id: Option<u32>,
}

fn default_span() -> u32 {
    1
}

impl AssetManifest {
    /// Parse a manifest from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest_with_sounds() {
        let json = r#"{
            "atlases": [],
            "sounds": {
                "click": { "path": "click.mp3", "event_id": 1 },
                "bg_music": { "path": "music.ogg" }
            }
        }"#;
        let manifest = AssetManifest::from_json(json).unwrap();
        assert_eq!(manifest.sounds.len(), 2);

        let click = &manifest.sounds["click"];
        assert_eq!(click.path, "click.mp3");
        assert_eq!(click.event_id, Some(1));

        let music = &manifest.sounds["bg_music"];
        assert_eq!(music.path, "music.ogg");
        assert_eq!(music.event_id, None);
    }

    #[test]
    fn parse_minimal_manifest() {
        let json = r#"{
            "atlases": [
                { "name": "tiles", "cols": 16, "rows": 8, "path": "tiles.png" }
            ],
            "sprites": {
                "hero": { "atlas": 0, "col": 0, "row": 0 }
            }
        }"#;
        let manifest = AssetManifest::from_json(json).unwrap();
        assert_eq!(manifest.atlases.len(), 1);
        assert_eq!(manifest.atlases[0].cols, 16);
        assert_eq!(manifest.sprites["hero"].atlas, 0);
    }
}
