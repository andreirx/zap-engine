# Architectural Decision Records

## ADR-001: Fat Entity over ECS

**Date:** 2026-02-05
**Status:** Accepted

### Context
We need an entity system for the engine. Full ECS (like Bevy's or Specs) provides maximum flexibility but adds complexity. Our target audience includes educational use and rapid prototyping.

### Decision
Use a "Fat Entity" model: a single `Entity` struct with optional components (`sprite`, `body`, `emitter`, `mesh`). Store entities in a flat `Vec<Entity>`.

### Consequences
- **Pro:** Simple to understand, no archetype tables or query systems
- **Pro:** Good cache locality for small entity counts (hundreds)
- **Pro:** Easy to extend with new optional fields
- **Con:** Wastes memory on unused component slots
- **Con:** Does not scale to millions of entities
- **Mitigation:** This engine targets web games with hundreds of entities, not MMOs

---

## ADR-002: SharedArrayBuffer Protocol

**Date:** 2026-02-05
**Status:** Accepted

### Context
The game simulation runs in a Web Worker (Rust/WASM). The main thread needs to read render data (sprite positions, effects vertices) every frame at 60fps.

### Decision
Use SharedArrayBuffer with Atomics for zero-copy data sharing. Fall back to `postMessage` with buffer copies when COOP/COEP headers are unavailable.

### Layout
```
[Header: 12 floats]
  0: lock (Atomics signal)
  1: frame_counter
  2: instance_count
  3: atlas_split
  4: effects_vertex_count
  5: world_width
  6: world_height
  7: sound_count
  8: event_count
  9-11: reserved

[Instance Data: 512 × 8 floats = 4096 floats]
  Per instance: x, y, rotation, scale, sprite_col, alpha, cell_span, atlas_row

[Effects Data: 16384 × 5 floats = 81920 floats]
  Per vertex: x, y, z(color_idx), u, v

[Sound Events: 32 floats]
  Per event: event_id (as f32)

[Game Events: 32 × 4 floats = 128 floats]
  Per event: kind, a, b, c
```

### Consequences
- **Pro:** Zero-copy reads at 60fps — no GC pressure
- **Pro:** Graceful fallback for non-COOP/COEP environments
- **Con:** Requires COOP/COEP headers for optimal path
- **Con:** Fixed buffer sizes (512 instances, 16K effects verts)

---

## ADR-003: Dual Rendering Backend (WebGPU + Canvas 2D)

**Date:** 2026-02-05
**Status:** Accepted

### Context
WebGPU enables HDR/EDR rendering but is not universally available (notably missing in Firefox as of 2026).

### Decision
Implement both WebGPU and Canvas 2D backends behind a common `Renderer` interface. Probe WebGPU on a disposable canvas first; fall back to Canvas 2D if unavailable.

### Consequences
- **Pro:** Works on all modern browsers
- **Pro:** HDR/EDR glow effects on capable hardware
- **Pro:** WebGPU probe avoids locking the real canvas
- **Con:** Canvas 2D path has lower visual fidelity (no procedural glow, no EDR)
- **Con:** Two code paths to maintain

---

## ADR-004: Manifest-Driven Asset Pipeline

**Date:** 2026-02-05
**Status:** Accepted

### Context
The original ZapZap codebase hardcoded atlas layouts (16×8 for base_tiles, 8×8 for arrows). The engine should work with any game's assets.

### Decision
Use a JSON `assets.json` manifest that describes atlases (name, cols, rows, path) and named sprites (atlas index, col, row). The renderer creates one pipeline per atlas with the correct `ATLAS_COLS`/`ATLAS_ROWS` overrides.

### Consequences
- **Pro:** Games define their own atlases without engine changes
- **Pro:** Named sprite lookup for developer convenience
- **Pro:** Pipeline-per-atlas maps cleanly to WebGPU constant overrides
- **Con:** Slightly more complex renderer initialization

---

## ADR-005: Scale Field = World-Space Size

**Date:** 2026-02-05
**Status:** Accepted

### Context
The original ZapZap shader had `let tile_size = 50.0 * inst.scale`, hardcoding a 50-unit tile size. This made the scale field a unitless multiplier.

### Decision
Remove the `50.0 *` from the shader. The `scale` field now represents the actual world-space rendered size. Games write the size directly (e.g., 50.0 for a 50-unit tile).

### Consequences
- **Pro:** No magic numbers in the shader
- **Pro:** Games have full control over rendered size
- **Con:** ZapZap migration requires changing scale from 1.0 to 50.0 for tiles
