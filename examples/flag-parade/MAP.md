# flag-parade/ — SDF Flag Waving Simulation

384 SDF spheres (24x16 grid) simulating a flag waving in the wind. Users pick from 10 country flags via a React side panel.

## Architecture

- **Pure SDF rendering** — no sprites, no vectors, no physics
- **Procedural animation** — sinusoidal wave (2 harmonics) propagating left→right
- **Depth shading** — wave troughs are darker (farther), peaks are brighter (closer)
- **Per-frame mutation** — entities updated in place via `scene.get_mut()`

## Files

| File | Purpose |
|------|---------|
| `src/game.rs` | `FlagParade` game struct — wave simulation + entity updates |
| `src/flags.rs` | 10 flag color definitions — `flag_color(flag, col, row)` |
| `src/lib.rs` | WASM exports (`GameRunner<FlagParade>`) |
| `App.tsx` | React UI — canvas + flag selector panel |

## Custom Events

| Kind | Direction | Meaning |
|------|-----------|---------|
| 1 | React → Rust | Select flag (a = flag index 0–9) |
