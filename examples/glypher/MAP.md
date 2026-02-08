# glypher/

Handwriting-tracing game built on ZapEngine. Players trace letter strokes to spell out proverbs on a dark, lit background.

## Architecture

**Game loop**: Sayings DB → pick saying → per-word → per-letter → per-stroke tracing → celebration → next.

**Rendering**: Dark textured background (with normal map) + dynamic PointLights + vector strokes (guide, user, celebration) + particles.

**Contextual alternates**: Lowercase letters have Baseline/High entry variants. Exit type of previous letter determines which variant to show next (e.g., after 'o' which has High exit, the next letter uses its High variant).

## Structure

| File | Purpose |
|---|---|
| `Cargo.toml` | Rust crate config — `zap-engine` + `zap-web` with vectors feature (no physics) |
| `src/lib.rs` | WASM exports wrapping `GameRunner<Glypher>` |
| `src/game.rs` | Game trait impl — state machine, rendering, input handling |
| `src/glyphs.rs` | Baked glyph JSON parser (serde) — character stroke data |
| `src/sayings.rs` | Proverbs/sayings database |
| `src/tracing.rs` | Stroke tracing validator — proximity + direction checks |
| `App.tsx` | React component using `useZapEngine` |
| `main.tsx` | Entry point — renders `<App />` |
| `index.html` | Vite HTML entry |
| `data/glyphs_baked.json` | Baked glyph stroke data (export from `tools/glyph-editor.html`) |
| `data/sayings.json` | Proverbs/sayings collection (JSON array) |
| `public/assets/` | Background texture + normal map + `assets.json` manifest |

## Data Flow

1. Baked glyphs + sayings are embedded in WASM via `include_str!()`
2. Parsed in `Game::init()` using serde
3. Game picks a saying → splits into words → letters → strokes
4. Each stroke's normalized [0,1] coords mapped to world coords
5. `StrokeTracer` validates user touch against reference path
6. Rendering: vectors (strokes) + lights (guide, user, celebration) + effects (particles)

## Coordinate System

- **World**: 800x600
- **Top zone** (y: 0–120): Word hints — underscores + completed mini-glyphs
- **Drawing zone** (y: 120–600): Glyph displayed centered, scaled by width class

## Build

```bash
wasm-pack build examples/glypher --target web --out-dir pkg
```
