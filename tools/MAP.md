# tools/

Build-time CLI utilities for the ZapEngine workflow.

## bake-assets.ts

Scans an image directory and generates an `assets.json` manifest compatible with the engine's `AssetManifest` format.

**Convention-based atlas detection:**
- `hero_4x8.png` → atlas "hero" with 4 columns, 8 rows → sprites `hero_0_0` through `hero_3_7`
- `background.png` → single-sprite atlas → sprite `background`

**Usage:**
```bash
npx tsx tools/bake-assets.ts path/to/images/ --output path/to/assets.json
```

## Architecture Connection

The baker's output feeds into `loadManifest()` (TypeScript) and the Rust `AssetManifest` parser. The schema is defined in `packages/zap-web/src/assets/manifest.ts` and `crates/zap-engine/src/assets/manifest.rs`.
