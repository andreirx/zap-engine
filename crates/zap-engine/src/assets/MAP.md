# assets/

Asset manifest parsing and sprite registry for named sprite lookups.

## Files

| File | Purpose |
|------|---------|
| `manifest.rs` | `AssetManifest`, `AtlasDescriptor`, `SpriteDescriptor`, JSON parsing |
| `registry.rs` | `SpriteRegistry` for `ctx.sprite("name")` lookups |

## Data Flow

```
assets.json (TypeScript)
    → postMessage → Worker
    → game_load_manifest(json)
    → SpriteRegistry::from_manifest()
    → ctx.sprite("hero")
```

## Key Types

- **`AssetManifest`**: Parsed JSON manifest containing atlases, sprites, sounds
- **`AtlasDescriptor`**: Atlas path, grid dimensions, optional normal map
- **`SpriteDescriptor`**: Named sprite with atlas index, column, row
- **`SpriteRegistry`**: HashMap lookup converting names → `SpriteComponent`

## Architecture Notes

The manifest is parsed once at initialization. Games reference sprites by name (`ctx.sprite("hero")`) instead of hardcoded atlas indices. The registry is private to `EngineContext`, exposed via the `sprite()` method.
