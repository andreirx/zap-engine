# examples/

Example games built with ZapEngine, demonstrating the engine's capabilities.

| Example | Purpose |
|---|---|
| `basic-demo/` | Minimal demo: spawns colored sprites that bounce around, responds to pointer input |
| `zap-engine-template/` | Starter template for creating new games |
| `react-demo/` | Demonstrates the `useZapEngine` React hook with a HUD overlay |
| `physics-playground/` | Angry Birds-style sling + tower with rapier2d physics, sprites, custom events |
| `chemistry-lab/` | SDF molecule builder: atoms as raymarched spheres, spring-joint bonds, zero-gravity |
| `zapzap-mini/` | 8x8 circuit puzzle with dynamic lighting, normal maps, and electric arcs |
| `glypher/` | New game built on ZapEngine (in development) |

## Structure

Each example follows the same pattern:

- `Cargo.toml` — Rust crate depending on `zap-engine` + `zap-web`
- `src/lib.rs` — `#[wasm_bindgen]` exports wrapping `GameRunner<MyGame>`
- `src/game.rs` — `Game` trait implementation
- `App.tsx` / `main.tsx` — React frontend using `@zap/web/react`
- `index.html` — Vite entry point
- `public/assets/` — Asset manifests and textures
- `pkg/` — wasm-pack output (gitignored)
