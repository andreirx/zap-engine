# ZapZap Mini — Example Game

Simplified 8x8 version of the ZapZap circuit puzzle game. Showcases dynamic lighting, normal maps, render layers, and electric arc effects.

## Architecture

```
src/
  lib.rs          — WASM exports (thread_local GameRunner, wasm-bindgen FFI)
  board.rs        — Grid, Tile, Rng, GameBoard, BFS connection checking, gravity
  animation.rs    — RotateAnim (tile tap), FallAnim (gravity), AnimationState
  game.rs         — ZapZapMini Game trait impl, state machine, rendering, lighting

public/assets/
  assets.json           — Asset manifest (atlas + normalMap reference)
  base_tiles.png        — 16x8 tile atlas (1024x512), copied from native xcassets
  base_tiles_normals.png — Generated normal map (Sobel, strength 2.0)

App.tsx           — React UI (score, new game button, FPS)
main.tsx          — React entry point
index.html        — HTML shell
```

## Game Flow

1. **WaitingForInput** — Player taps a tile
2. **RotatingTile** — Tile rotates 90 degrees CCW (0.2s animation)
3. Connection check via two-pass BFS flood-fill
4. If left-to-right path found → score tiles, spawn particles + arcs
5. **FreezeDuringZap** — Display arcs + lights for 2 seconds
6. Remove Ok-marked tiles, gravity shifts remaining down
7. **FallingTiles** — Gravity animation until settled
8. Back to WaitingForInput

## Key Features Demonstrated

- **Dynamic Lighting**: Point lights at connected tile positions with per-frame wiggle
- **Normal Maps**: Bump-mapped tile shadows from arc lighting
- **Render Layers**: Background/Terrain/Objects/VFX layer separation
- **Electric Arcs**: Engine's `add_arc()` with multiple colors per marking type
- **Particles**: Zap explosion particles via `spawn_particles()`
- **Custom Events**: React "New Game" button → Rust via `InputEvent::Custom`
- **Game Events**: Score updates → React via `GameEvent`
