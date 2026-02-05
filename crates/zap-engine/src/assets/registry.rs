use std::collections::HashMap;
use crate::assets::manifest::AssetManifest;
use crate::components::sprite::{SpriteComponent, AtlasId, BlendMode};

/// Registry of named sprites, built from an AssetManifest.
/// Provides convenient name-based sprite lookup for game code.
pub struct SpriteRegistry {
    sprites: HashMap<String, SpriteComponent>,
}

impl SpriteRegistry {
    pub fn new() -> Self {
        Self {
            sprites: HashMap::new(),
        }
    }

    /// Build a registry from a parsed AssetManifest.
    pub fn from_manifest(manifest: &AssetManifest) -> Self {
        let mut sprites = HashMap::with_capacity(manifest.sprites.len());
        for (name, desc) in &manifest.sprites {
            sprites.insert(name.clone(), SpriteComponent {
                atlas: AtlasId(desc.atlas),
                col: desc.col as f32,
                row: desc.row as f32,
                cell_span: desc.span as f32,
                alpha: 1.0,
                blend: BlendMode::Alpha,
            });
        }
        Self { sprites }
    }

    /// Look up a sprite by name. Returns None if not found.
    pub fn get(&self, name: &str) -> Option<&SpriteComponent> {
        self.sprites.get(name)
    }
}

impl Default for SpriteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_from_manifest() {
        let json = r#"{
            "atlases": [
                { "name": "tiles", "cols": 16, "rows": 8, "path": "tiles.png" }
            ],
            "sprites": {
                "hero": { "atlas": 0, "col": 3, "row": 5, "span": 2 }
            }
        }"#;
        let manifest = AssetManifest::from_json(json).unwrap();
        let reg = SpriteRegistry::from_manifest(&manifest);

        let hero = reg.get("hero").expect("hero should exist");
        assert_eq!(hero.atlas, AtlasId(0));
        assert_eq!(hero.col, 3.0);
        assert_eq!(hero.row, 5.0);
        assert_eq!(hero.cell_span, 2.0);
        assert_eq!(hero.alpha, 1.0);
    }

    #[test]
    fn unknown_returns_none() {
        let reg = SpriteRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }
}
