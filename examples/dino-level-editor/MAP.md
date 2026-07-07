# examples/dino-level-editor/

Playable dinosaur platformer level editor example.

## Architecture Connection

This example follows the ZapEngine headless pattern from `docs/VISION.md`: Rust owns the level state, edit/play state machine, collision, save-document serialization, camera transform, and render command generation; React is the control surface and browser-API adapter for browser persistence. The example intentionally keeps the editor model local to this crate rather than introducing a reusable editor framework, because there is one current caller and no ratified cross-example editor API yet.

## Structure

| Path | Purpose |
|---|---|
| `Cargo.toml` | WASM crate depending on `zap-engine` and `zap-web` with vector drawing enabled |
| `src/` | Rust level/editor/play logic and wasm-bindgen exports |
| `App.tsx` | React toolbar/HUD; sends custom events to Rust and receives mode/score/life events |
| `main.tsx` | React entry point |
| `index.html` | Vite HTML shell |
| `public/assets/` | Copied dinosaur, tile, hazard, coin, and finish PNG assets plus `assets.json` |

## Behavior

- Level grid is 64×32 tiles, each tile 128×128 world units.
- Edit mode fits the full level in view; play mode animates to a zoomed camera that follows forward movement only.
- The default browser-storage slot auto-loads when the engine becomes ready and auto-saves while editing. Save/Load buttons are for additional named slots, preferring IndexedDB, falling back to localStorage, then to current-session memory if the browser shell exposes neither storage API.
- Reset is edit-only and requires an explicit Yes/Cancel confirmation.
- Rectangle editing shows a translucent vector preview while dragging, green for place and red for erase.
- Ground surface selection is derived from topology: a ground tile with no ground above uses `pamant`; buried tiles use deterministic pseudo-random underground variants.
- Lava and water are tile materials in the same 64×32 grid. Lava is non-solid but lethal in play mode; water is non-solid and applies viscous swim-like movement.
- `carne` is a one-tile powerup object. In play mode it doubles the dinosaur's visible width and height and gives one fireball hit buffer; the buffered hit shrinks the dinosaur back to normal instead of ending play.
- Dino start is continuous world-space constrained onto the ground surface, not tile-snapped.
- Finish gate is split into back (`finish-spate-square`) and front (`finish-fata-square`) sprites so the dinosaur can render between them.
- `vulcan` is a movable 1024×1024 background sprite rendered as 8×8 tiles behind terrain.
- Fireballs and smoke are sprite-driven hazards: crater fireballs animate in edit/play; timed eruptions disappear on any tile material and only collide with the dinosaur in play mode.

## Run

```bash
wasm-pack build examples/dino-level-editor --target web --out-dir pkg
npm run dev
```

Open `/examples/dino-level-editor/index.html`.
