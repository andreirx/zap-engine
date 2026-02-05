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
See ADR-006 for the current header layout (16 floats, self-describing with capacities).
```
[Header: 16 floats — see ADR-006]

[Instance Data: max_instances × 8 floats]
  Per instance: x, y, rotation, scale, sprite_col, alpha, cell_span, atlas_row

[Effects Data: max_effects_vertices × 5 floats]
  Per vertex: x, y, z(color_idx), u, v

[Sound Events: max_sounds floats]
  Per event: event_id (as f32)

[Game Events: max_events × 4 floats]
  Per event: kind, a, b, c
```

### Consequences
- **Pro:** Zero-copy reads at 60fps — no GC pressure
- **Pro:** Graceful fallback for non-COOP/COEP environments
- **Pro:** Configurable buffer sizes via GameConfig (see ADR-006)
- **Con:** Requires COOP/COEP headers for optimal path

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

---

## ADR-006: Configurable Buffer Capacities

**Date:** 2026-02-05
**Status:** Accepted

### Context
Buffer capacities (`MAX_INSTANCES=512`, `MAX_EFFECTS_VERTICES=16384`, `MAX_SOUNDS=32`, `MAX_EVENTS=32`) were hardcoded as compile-time constants, duplicated across Rust `protocol.rs`, TypeScript `protocol.ts`, and locally in `webgpu.ts`. A puzzle game and a bullet-hell game got the same fixed allocation.

### Decision
Make capacities runtime-configurable via `GameConfig`. The SharedArrayBuffer header is self-describing: capacities are written once at init (slots 2, 5, 9, 11), and TypeScript reads them from the header to compute offsets dynamically via the `ProtocolLayout` class.

**Header redesign (12 → 16 floats):**
Each capacity is interleaved with its per-frame count for locality:
```
 0: lock                      (per-frame, Int32 Atomics)
 1: frame_counter             (per-frame)
 2: max_instances             (once — capacity)
 3: instance_count            (per-frame)
 4: atlas_split               (per-frame)
 5: max_effects_vertices      (once — capacity)
 6: effects_vertex_count      (per-frame)
 7: world_width               (per-frame)
 8: world_height              (per-frame)
 9: max_sounds                (once — capacity)
10: sound_count               (per-frame)
11: max_events                (once — capacity)
12: event_count               (per-frame)
13: protocol_version (1.0)    (once)
14-15: reserved
```

**Wire-format constants remain fixed:**
`INSTANCE_FLOATS=8`, `EFFECTS_VERTEX_FLOATS=5`, `EVENT_FLOATS=4`, `HEADER_FLOATS=16`.

### Consequences
- **Pro:** Zero duplication — single source of truth in `GameConfig`
- **Pro:** Games can tune allocations (puzzle: fewer instances; bullet-hell: more)
- **Pro:** Self-describing header enables forward compatibility
- **Pro:** Default values preserve backward compatibility
- **Con:** Header grew from 12 to 16 floats (+16 bytes)

---

## ADR-007: Rapier2D Physics Integration

**Date:** 2026-02-05
**Status:** Accepted

### Context
The engine needs 2D rigid-body physics (gravity, collisions, bounce) for games. The basic-demo was using manual velocity/bounce logic, which doesn't scale to real game physics.

### Decision
Integrate `rapier2d` v0.22 as a feature-gated (`physics`, default on) dependency of `zap-engine`.

**Key design choices:**

1. **`PhysicsWorld` wrapper**: Encapsulates all 9 Rapier struct instances (pipeline, bodies, colliders, broad/narrow phase, island manager, joints, CCD solver, query pipeline). Games interact through a clean API without touching Rapier directly.

2. **No nalgebra in public API**: All public types use `glam::Vec2`. Internal conversion functions (`vec2_to_na`, `na_to_vec2`, `na_iso_to_pos_rot`) handle the bridging.

3. **WASM-safe event collection**: Custom `DirectEventCollector` implements `EventHandler` using `RefCell<Vec>` instead of `ChannelEventCollector` (which depends on crossbeam, which doesn't compile to `wasm32-unknown-unknown`).

4. **EntityId stored in `user_data`**: Each Rapier body stores the `EntityId` as `u128` in its `user_data` field, enabling collision event resolution back to game entities.

5. **`step_into(&mut Vec<CollisionPair>)` pattern**: The simulation step writes collision events into an external Vec, avoiding borrow conflicts between the physics world and event iteration.

6. **Entity `despawn` via EngineContext**: `ctx.despawn(id)` cleans up both the Scene entity and its Rapier body+colliders in one call.

**Game loop order (per fixed step):**
```
1. game.update()       — apply forces, spawn, read PREVIOUS step's collisions
2. ctx.step_physics()  — Rapier step + position sync to entities + collect events
3. effects.tick(dt)    — particles/arcs see updated positions
```

### Consequences
- **Pro:** Feature-gated — games without physics pay zero cost (no rapier2d compiled)
- **Pro:** Clean API — games use `ctx.spawn_with_body()`, `ctx.apply_impulse()`, `ctx.collisions()`
- **Pro:** Automatic position sync — no manual `entity.pos = physics.get_pos()` needed
- **Pro:** WASM-compatible — custom event handler avoids crossbeam dependency
- **Con:** rapier2d adds ~500KB to WASM binary (acceptable for physics-enabled games)
- **Con:** One-frame collision event delay (events from step N visible in step N+1)
