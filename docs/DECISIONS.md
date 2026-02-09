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

[SDF Instances: max_sdf_instances × 12 floats]
  Per instance: x, y, radius, rotation, r, g, b, shininess, emissive, shape_type, half_height, extra
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
14: max_sdf_instances          (once — capacity)
15: sdf_instance_count          (per-frame)
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

---

## ADR-008: Entity-Attached Emitter System

**Date:** 2026-02-05
**Status:** Accepted

### Context
Games need particle effects tied to entities (trails, auras, exhausts). The existing `EffectsState` provides manual `spawn_particles()` but requires explicit calls from game code each frame.

### Decision
Add an `EmitterComponent` that attaches to entities and auto-spawns particles:

1. **`EmitterComponent`** with configurable emission mode (continuous/burst), rate, speed range, lifetime, color mode, and per-particle physics (drag, attract_strength, speed_factor).
2. **`Particle` extended** with per-instance physics fields (previously hardcoded constants), backward-compatible defaults.
3. **`tick_emitters()` free function** iterates entities, ticks their emitters, and spawns particles into `EffectsState`. Free function pattern avoids borrow conflicts between `Scene` and `EffectsState`.
4. **Game loop position:** After physics step, before `effects.tick()`.

**Emission modes:**
- `Continuous`: accumulator-based, spawns `rate × dt` particles per frame.
- `Burst`: one-shot (interval=0) or repeating (interval>0).

### Consequences
- **Pro:** Declarative particle effects — attach once, auto-spawns
- **Pro:** Configurable per-particle physics — games can tune drag, attraction, speed
- **Pro:** Backward compatible — existing `spawn_particles()` uses default physics values
- **Pro:** Builder pattern for ergonomic configuration
- **Con:** Adds one `Option<EmitterComponent>` to the fat entity struct

---

## ADR-009: SDF Molecule Rendering Pipeline

**Date:** 2026-02-05
**Status:** Accepted

### Context
The chemistry visualization layer needs 3D-looking spheres (atoms/molecules) rendered efficiently in a 2D game engine. Traditional sprite-based circles lack depth cues (lighting, specular, Fresnel rim).

### Decision
Implement a raymarched sphere SDF pipeline:

1. **`MeshComponent`** on entities: shape (Sphere{radius}), color (RGB), shininess (Phong exponent), emissive (HDR glow multiplier).
2. **`SDFInstance`** buffer: 12 floats / 48 bytes per instance (x, y, radius, rotation, r, g, b, shininess, emissive, pad×3). `#[repr(C)]` + Pod/Zeroable for safe SharedArrayBuffer transfer.
3. **`molecule.wgsl` shader**: Instanced quads with per-fragment sphere raymarching, Phong shading (ambient + diffuse + specular), Fresnel rim glow, HDR emissive multiplier, edge anti-aliasing via smoothstep, discard outside sphere.
4. **WebGPU pipeline**: Separate shader module, storage buffer, bind group. Draw order: sprites → SDF → effects (SDF before additive effects so glows appear on top).
5. **Canvas2D fallback**: Filled circles with radial gradient (white highlight → base color → dark edge).
6. **Protocol extension**: Uses former reserved header slots (14, 15) for `max_sdf_instances` and `sdf_instance_count`. SDF data section appended after events in the SharedArrayBuffer layout.

**Draw order (final):**
```
sprites → SDF molecules → effects (additive glow)
```

### Consequences
- **Pro:** 3D-looking spheres with proper lighting in a 2D engine
- **Pro:** HDR/EDR emissive glow on capable displays
- **Pro:** Graceful Canvas2D fallback (radial gradient circles)
- **Pro:** Uses reserved header slots — no header size change
- **Pro:** SharedArrayBuffer grows only when `max_sdf_instances > 0`
- **Con:** Per-fragment raymarching is more expensive than textured quads
- **Con:** Only Sphere shape initially (extendable via SDFShape enum)

---

## ADR-010: React Hook Architecture

**Date:** 2026-02-05
**Status:** Accepted

### Context
The VISION.md specifies React as a first-class citizen: "DX: React is a first-class citizen. UI is HTML/CSS, not a canvas overlay." Games need a simple way to embed the engine in a React app without understanding workers, SharedArrayBuffer, or WebGPU internals.

### Decision
Provide a `useZapEngine` React hook that encapsulates the entire engine lifecycle:

1. **Separate import path**: `@zap/web/react` — the core engine (`@zap/web`) remains React-free, so non-React consumers (vanilla TS, Svelte, Vue) are not forced to depend on React.

2. **Single hook API**: `useZapEngine({ wasmUrl, assetsUrl, ... })` returns `{ canvasRef, sendEvent, fps, isReady, canvasKey }`.

3. **Canvas remount pattern**: When WebGPU init fails after tainting the canvas, the hook increments `canvasKey`. The consumer uses this as a React `key` prop on `<canvas>`, forcing React to unmount/remount a fresh DOM element, then retries with Canvas 2D.

4. **Input forwarding**: Pointer events (down/up/move) and keyboard events are forwarded to the worker. Sound manager resume is triggered on first pointer interaction.

5. **ResizeObserver**: Replaces manual `window.resize` listener. Observes the canvas element directly for more reliable sizing in flex/grid layouts.

### Consequences
- **Pro:** One-line engine integration for React apps
- **Pro:** Core engine stays framework-agnostic
- **Pro:** Canvas remount handles WebGPU fallback transparently
- **Pro:** FPS and ready state exposed as React state for HUD overlays
- **Con:** React is a devDependency even if only some consumers use the hook
- **Con:** `canvasKey` pattern requires consumer to pass it as `key` prop

---

## ADR-011: Convention-Based Asset Baker

**Date:** 2026-02-05
**Status:** Accepted

### Context
The VISION.md describes an asset pipeline: "Drop images into `assets/`, run `npm run bake-assets`, use string IDs in Rust." The MASTERPLAN references an `extract_assets.py` script to enhance, but no such script exists.

### Decision
Create a Node.js/TypeScript CLI tool (`tools/bake-assets.ts`) that scans a directory and outputs an `assets.json` manifest:

1. **Convention-based atlas detection**: Files named `*_NxM.ext` (e.g., `hero_4x8.png`) are treated as atlases with N columns and M rows. Files without this suffix are single-sprite atlases (1×1).

2. **No image processing**: The baker only catalogs files — no packing, resizing, or compression. This keeps it zero-dependency (only Node.js built-ins).

3. **Named sprite generation**: Single-sprite files get the filename (sans extension) as their sprite name. Multi-cell atlases get `{name}_{col}_{row}` entries.

4. **Separate tsconfig**: Tools run under Node.js (not the browser), so `tools/tsconfig.json` has `@types/node` in its types array, isolated from the main browser tsconfig.

### Consequences
- **Pro:** Zero dependencies — runs with `npx tsx`, no install needed
- **Pro:** Convention over configuration — no manual manifest authoring for simple cases
- **Pro:** Output matches the existing `AssetManifest` JSON schema exactly
- **Con:** Naming convention is the only way to specify atlas dimensions (no config file fallback)
- **Con:** No atlas packing — games with many small sprites still need manual packing

---

## ADR-012: Debug Rendering via Effects Pipeline

**Date:** 2026-02-05
**Status:** Accepted

### Context
When developing physics-based games, visualizing collider shapes (hitboxes) is essential for debugging. The engine already has an effects pipeline (5-float vertices, `build_strip_vertices()` → `strip_to_triangles()`, additive glow shader) that renders line-like geometry.

### Decision
Reuse the existing effects pipeline for debug collider visualization:

1. **`collider_shape()` on PhysicsWorld**: Queries Rapier's collider set to extract `ColliderDesc` from handles — games never touch Rapier directly.

2. **`DebugLine` in EffectsState**: `debug_lines: Vec<DebugLine>` field with `add_debug_line(points, width, color)` and `clear_debug()`. Debug lines are included in `rebuild_effects_buffer()` after arcs and particles, using the same strip→triangle pipeline.

3. **`debug_draw_colliders()` free function**: Takes separate `&Scene`, `&PhysicsWorld`, `&mut EffectsState` references (same borrow-conflict-avoidance pattern as `tick_emitters()`). Generates outlines: 24-segment circles for Ball, rotated rectangles for Cuboid, semicircles+sides for CapsuleY.

4. **Opt-in per frame**: Games call `debug_draw_colliders()` from `update()` — no overhead when not used.

### Consequences
- **Pro:** Zero new GPU resources — reuses the existing additive effects pipeline
- **Pro:** Supports all collider shapes (Ball, Cuboid, CapsuleY)
- **Pro:** Feature-gated — compiles out entirely when physics feature is disabled
- **Pro:** Opt-in per frame — no performance cost when not debugging
- **Con:** Debug lines use additive blend (glow) — not ideal for opaque wireframes, but visually distinctive

---

## ADR-013: Tier-Aware HDR Rendering

**Date:** 2026-02-05
**Status:** Accepted

### Context
The WebGPU renderer configures surfaces in a 3-tier fallback cascade (rgba16float + display-p3 + extended tone mapping → rgba16float + sRGB → preferred format). However, shaders hardcoded HDR glow multipliers (6.4 for effects, 5.4 for SDF emissive), meaning SDR-tier displays would receive oversaturated/clamped output.

### Decision
Introduce a `RenderTier` type and per-tier shader constants:

1. **`RenderTier`**: `'hdr-edr' | 'hdr-srgb' | 'sdr' | 'canvas2d'` — exposed on the `Renderer` interface so games/UI can adapt (e.g., show "HDR" badge, adjust bloom).

2. **WGSL override constants**: `EFFECTS_HDR_MULT` in `shaders.wgsl` and `SDF_EMISSIVE_MULT` in `molecule.wgsl` — default to full EDR values (6.4, 5.4), overridden at pipeline creation per tier:
   - `hdr-edr`: 6.4 / 5.4 (full EDR range)
   - `hdr-srgb`: 3.0 / 2.5 (HDR within sRGB gamut)
   - `sdr`: 1.0 / 0.5 (safe for bgra8unorm)

3. **Tier-based resize**: The `resize()` function uses the negotiated tier to reconfigure the canvas (only `hdr-edr` gets `display-p3` + `extended` tone mapping).

### Consequences
- **Pro:** No more clamping/oversaturation on SDR displays
- **Pro:** Gradual degradation — each tier gets the best possible visual quality
- **Pro:** `RenderTier` exposed to consumers for adaptive UI
- **Pro:** Uses existing WebGPU pipeline override constants — zero runtime overhead
- **Con:** Adds a lookup table for per-tier values (6 numbers total)

---

## ADR-014: Audio System Completion

**Date:** 2026-02-05
**Status:** Accepted

### Context
The engine had a working `SoundManager` (Web Audio API, play by event ID, background music), but the manifest-to-config bridge was missing. Games had to manually construct `SoundConfig` despite the manifest already declaring sounds with `event_id` fields. Per-sound volume control was also absent.

### Decision
Complete the audio pipeline with three additions:

1. **Per-sound volume via `SoundEntry`**: `SoundConfig.sounds` accepts `string | SoundEntry` where `SoundEntry = { path, volume? }`. Playback routes through a `GainNode` when `volume < 1.0`. Backward compatible — plain strings still work (volume defaults to 1.0).

2. **`buildSoundConfigFromManifest()` helper**: Bridges `AssetManifest.sounds` (which has `path` + optional `event_id`) to `SoundConfig`. Iterates manifest entries, maps those with `event_id` to the sounds record. Zero-config audio for manifest-driven games.

3. **Eager `init()` in React hook**: `SoundManager.init()` is called immediately after construction (before user interaction). AudioContext starts suspended; existing `resume()` on `pointerdown` handles unsuspension. This pre-decodes audio buffers so first-play latency is eliminated.

### Consequences
- **Pro:** Manifest-driven games get audio with zero manual config
- **Pro:** Per-sound volume enables mix control (quiet UI clicks, loud explosions)
- **Pro:** Eager init eliminates first-play latency
- **Pro:** Fully backward compatible — no breaking changes to SoundConfig
- **Con:** Audio buffers are decoded even if never played (acceptable — they're typically small)

---

## ADR-015: Extended SDF Shapes for Chemistry Visualization

**Date:** 2026-02-05
**Status:** Accepted

### Context
The SDF molecule pipeline (ADR-009) only supported spheres. For chemistry/educational apps, we need capsule shapes (bonds between atoms) and rounded boxes (labels, indicators). The `SDFInstance` struct had 3 padding fields (`_pad0`, `_pad1`, `_pad2`) occupying 12 bytes of the 48-byte wire format.

### Decision
Repurpose the padding fields to encode shape parameters — **no protocol or buffer size changes**:

1. **`_pad0` → `shape_type`**: 0.0 = Sphere, 1.0 = Capsule, 2.0 = RoundedBox. Float thresholds in the shader (< 0.5 sphere, < 1.5 capsule, else box) avoid integer comparison issues.

2. **`_pad1` → `half_height`**: Cylinder half-length for Capsule, box half-height for RoundedBox. 0.0 for Sphere.

3. **`_pad2` → `extra`**: Corner radius for RoundedBox. 0.0 for Sphere/Capsule.

**Backward compatible**: Existing zeroed padding encodes shape_type = 0.0 = Sphere.

**Shader changes (`molecule.wgsl`)**:
- SDF primitive functions: `sdf_sphere()`, `sdf_capsule()`, `sdf_rounded_box()`
- Normal estimation: Sphere uses analytic normals (fast), Capsule/RoundedBox use central-difference gradient
- Vertex shader applies entity rotation and elongates quads for non-sphere shapes
- Same Phong + Fresnel + HDR shading pipeline for all shapes

**Canvas 2D fallback**: Capsule/RoundedBox drawn as rotated rounded rectangles with linear gradients.

### Consequences
- **Pro:** No protocol change — 48 bytes / 12 floats preserved
- **Pro:** Fully backward compatible — zeroed fields = Sphere
- **Pro:** Capsules model bonds between atoms naturally
- **Pro:** RoundedBox enables labels and periodic-table-style indicators
- **Pro:** Same Phong + Fresnel + HDR shading for all shapes — visual consistency
- **Con:** Capsule/RoundedBox normals use central-difference (4 extra SDF evaluations per fragment)
- **Con:** Two more shape types to maintain in both WebGPU and Canvas 2D code paths

---

## ADR-016: Sprite Registry (Manifest → EngineContext Bridge)

**Date:** 2026-02-05
**Status:** Accepted

### Context
The `AssetManifest` contains a `sprites: HashMap<String, SpriteDescriptor>` with named sprite definitions, but this data never reached Rust game code. Games had to hardcode atlas indices, column/row numbers.

### Decision
Add `SpriteRegistry` to the engine that converts manifest sprite descriptors into ready-to-use `SpriteComponent` objects. The manifest JSON is passed from TypeScript → Worker → WASM during initialization.

**Data flow:** React hook captures manifest JSON → Worker receives `manifestJson` in init message → calls `game_load_manifest(json)` → `GameRunner::load_manifest()` → `EngineContext::load_manifest()` → `SpriteRegistry::from_manifest()`.

**API:** `ctx.sprite("hero")` returns `Option<SpriteComponent>` — a clone ready to attach to an entity.

### Consequences
- **Pro:** Games reference sprites by name ("hero", "block_red") instead of magic numbers
- **Pro:** Zero runtime cost after init — HashMap lookup only during `sprite()` calls
- **Pro:** Backward compatible — `load_manifest` is optional (game_load_manifest export is optional)
- **Con:** Manifest JSON is serialized, sent over postMessage, then re-parsed in WASM (one-time cost)

---

## ADR-017: Joints API (Fixed, Spring, Revolute)

**Date:** 2026-02-05
**Status:** Accepted

### Context
Rapier 0.22 includes `ImpulseJointSet` with `FixedJointBuilder`, `SpringJointBuilder`, and `RevoluteJointBuilder`, but `PhysicsWorld` only exposed body creation and forces. The Chemistry Lab example needs spring joints for molecular bonds.

### Decision
Expose joints through a clean `JointDesc` enum + `JointHandle` wrapper, hiding Rapier internals.

```rust
pub enum JointDesc {
    Fixed { anchor_a: Vec2, anchor_b: Vec2 },
    Spring { anchor_a: Vec2, anchor_b: Vec2, rest_length: f32, stiffness: f32, damping: f32 },
    Revolute { anchor_a: Vec2, anchor_b: Vec2 },
}
```

**PhysicsWorld methods:** `create_joint()`, `remove_joint()`, `joint_count()`.
**EngineContext convenience:** `create_joint(entity_a, entity_b, desc)` looks up both entities' physics bodies.

### Consequences
- **Pro:** Clean API using `glam::Vec2` — no nalgebra exposure
- **Pro:** Three most common 2D joint types covered
- **Pro:** Games can extend with more joint types by accessing `physics.impulse_joints` directly
- **Con:** Rope and Prismatic joints not wrapped yet (can be added when needed)
- **Con:** No motor/limit API exposed — would need builder pattern extension

---

## ADR-018: Render Layers (Photoshop-Style Compositing)

**Date:** 2026-02-06
**Status:** Accepted

### Context
All entities rendered on the same draw layer — no control over draw order beyond atlas grouping. Games need background/terrain behind objects, UI on top, VFX between layers.

### Decision
Add a `RenderLayer` enum with 6 layers (Background=0 through UI=5), stored directly on Entity. Entities are sorted by `(layer, atlas)` during `build_render_buffer()`. Layer batch descriptors are written to the SAB so the renderer draws each layer's instances in order.

**Key choices:**

1. **Layer info on Entity, NOT RenderInstance**: Keeps the wire format at 8 floats / 32 bytes. The sorting happens in Rust; the renderer just gets pre-sorted instance data plus batch descriptors.

2. **Default on (not feature-gated)**: All games benefit from layered rendering. Existing games default all entities to `Objects` layer — single batch, identical to the old behavior.

3. **Protocol 2.0 → 3.0**: Header extends from 18 → 22 floats with layer batch metadata (max_layer_batches, batch_count, batch_data_offset, reserved). New `LayerBatch` section appended after Vectors in the SAB.

4. **Batch descriptors in SAB header**: Each batch is 4 floats (layer_id, start, end, atlas_split). The renderer reads these to issue per-batch draw calls with the correct atlas pipeline.

5. **Backward compatible**: TypeScript detects protocol version and falls back to legacy atlas_split when no layer batches are present.

**Layer enum:**
```
Background = 0  (parallax, sky)
Terrain    = 1  (tiles, ground)
Objects    = 2  (default — game entities)
Foreground = 3  (decorations in front of objects)
VFX        = 4  (particle effects rendered as sprites)
UI         = 5  (HUD elements)
```

### Consequences
- **Pro:** Clean draw order control without Z-buffer complexity
- **Pro:** Zero protocol overhead for existing single-layer games (one batch)
- **Pro:** No RenderInstance format change — 32-byte wire format preserved
- **Pro:** Foundation for Phase 2 layer baking (render static layers to textures)
- **Con:** Header grew from 18 → 22 floats (+16 bytes)
- **Con:** Sort step adds O(n log n) to render buffer build (acceptable for hundreds of entities)

## ADR-019: Layer Baking (Render-to-Texture Caching)

**Date:** 2026-02-06
**Status:** Accepted

### Context
Static layers (e.g., terrain with hundreds of tiles) waste GPU work by re-rendering every frame. Games need a way to cache static layers as textures and only re-render them when content changes.

### Decision
Add `bake_layer()` / `invalidate_layer()` / `unbake_layer()` API on `EngineContext`. The bake state (a bitmask of baked layers + a monotonic generation counter) is encoded into SAB header[21] and communicated to the renderer.

**Key choices:**

1. **Bake state encoding**: `header[21] = baked_mask | (bake_generation << 6)` packed as a single f32. The mask uses bits 0-5 (6 layers), and the generation uses bits 6+ (up to ~262k invalidations before wrapping, which triggers a full re-render — acceptable).

2. **Generation-based dirty detection**: Rather than per-layer dirty tracking, a single monotonic `bake_generation` counter increments on every `bake_layer()` or `invalidate_layer()` call. The compositor compares its cached generation per-layer and re-renders when mismatched. This simplifies the protocol (no separate dirty mask) at the cost of occasionally re-rendering unrelated baked layers.

3. **WebGPU compositor**: New `LayerCompositor` class manages intermediate `GPUTexture` render targets per baked layer. Dirty layers are rendered in a separate render pass before the main composite pass. Clean layers are blitted via a fullscreen triangle with the `composite.wgsl` shader.

4. **Canvas2D fallback**: Uses `OffscreenCanvas` per baked layer. Dirty layers render to the offscreen canvas; clean layers are composited with `drawImage()`.

5. **Game-driven baking**: The game calls `ctx.bake_layer(RenderLayer::Terrain)` explicitly — the engine does not auto-detect static layers. This gives games full control over the performance/memory tradeoff.

**Usage:**
```rust
fn init(&mut self, ctx: &mut EngineContext) {
    // Spawn terrain tiles...
    ctx.bake_layer(RenderLayer::Terrain); // Cache once

    // Spawn moving objects on Objects layer (default, not baked)
}

fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
    if terrain_changed {
        ctx.invalidate_layer(RenderLayer::Terrain);
        ctx.bake_layer(RenderLayer::Terrain);
    }
}
```

### Consequences
- **Pro:** Static terrain with 1000+ tiles renders in a single texture blit instead of 1000 draw calls
- **Pro:** Zero overhead for games that don't use baking (bake_state remains 0)
- **Pro:** No header extension needed — reuses reserved slot [21]
- **Pro:** Works on both WebGPU (GPU textures) and Canvas2D (OffscreenCanvas)
- **Con:** Each baked layer costs one GPU texture of screen size (~16MB at 1080p rgba16float)
- **Con:** Generation-based dirty tracking may cause unnecessary re-renders of other baked layers on invalidation (acceptable for rare events like terrain edits)

## ADR-020: Dynamic Point Light System

**Date:** 2026-02-06
**Status:** Accepted

### Context
2D games benefit from dynamic lighting for atmosphere (torches, explosions, day/night). We need a lighting system that integrates with the existing render layer and baking infrastructure.

### Decision
Implemented a 2D point light system with fullscreen post-process lighting:

- **PointLight struct** (8 floats / 32 bytes, `#[repr(C)]`): `x, y, r, g, b, intensity, radius, layer_mask`
- **LightState**: Persistent lights (not cleared per-frame) with ambient RGB color
- **Protocol extension**: Header 22 → 28 floats (MAX_LIGHTS, LIGHT_COUNT, AMBIENT_R/G/B, reserved), protocol version 4.0
- **New SAB section**: `[Lights: max_lights × 8 floats]` after LayerBatches
- **Lighting shader** (`lighting.wgsl`): Fullscreen triangle post-process with smooth quadratic falloff `(1 - d/r)^2`
- **Two-target rendering**: When lighting active, scene renders to scratch texture first, then lighting pass composites to screen
- **Default ambient**: `(1.0, 1.0, 1.0)` — full white produces unlit output (backward compatible)

### Alternatives Considered
- **Per-layer lighting pass**: More accurate layer_mask support but significantly more complex. Deferred to a future iteration.
- **Deferred shading with G-buffer**: Overkill for 2D; the fullscreen post-process approach is simpler and sufficient.
- **Clearing lights each frame**: Rejected — persistent lights match the entity model (spawn once, update position) and avoid boilerplate.

### Usage
```rust
fn init(&mut self, ctx: &mut EngineContext) {
    ctx.lights.set_ambient(0.1, 0.1, 0.15);
    ctx.lights.add(PointLight::new(Vec2::new(400.0, 300.0), [1.0, 0.8, 0.6], 2.0, 200.0));
}

fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
    // Move light with entity
    for light in ctx.lights.iter_mut() {
        light.x = player_pos.x;
        light.y = player_pos.y;
    }
}
```

### Consequences
- **Pro:** Zero visual change for games that don't use lighting (ambient defaults to white)
- **Pro:** HDR-compatible — intensity > 1.0 produces natural bloom on HDR/EDR displays
- **Pro:** Persistent lights avoid per-frame allocation; retain/clear for lifecycle management
- **Pro:** layer_mask field ready for future per-layer lighting
- **Con:** Fullscreen post-process adds one extra render pass when lights are active
- **Con:** Scratch texture costs ~16MB at 1080p rgba16float when lighting is active

## ADR-021: Normal Map Pipeline (Offline Generation + Deferred Buffer)

**Date:** 2026-02-06
**Status:** Accepted

### Context
Flat per-pixel lighting (Phase 3) makes surfaces look uniform. Normal maps add per-pixel directional shading — bumps, grooves, and surface detail that respond to light direction.

### Decision
Implemented a two-part normal map system: offline generation tool + runtime deferred normal buffer.

**Offline Generation:**
- `tools/generate_normals.py`: Python script using Sobel operator on alpha or luminance channels
- Outputs RGBA normal map: `R=nx*0.5+0.5, G=ny*0.5+0.5, B=nz*0.5+0.5, A=source_alpha`
- Configurable strength multiplier and height source (alpha/luminance)

**Asset Pipeline:**
- `AtlasDescriptor` extended with optional `normalMap` / `normal_map` field (TS/Rust)
- `loadNormalMapBlobs()` fetches normal map PNGs in parallel with atlas PNGs
- Normal maps loaded WITHOUT premultiplied alpha to preserve raw normal values
- Flat normal placeholder texture (1×1, `(128,128,255,255)` = (0,0,1)) for atlases without normal maps

**Runtime Rendering (deferred normal buffer):**
- `fs_normal` fragment entry point in `shaders.wgsl`: outputs normal atlas texel with alpha blending
- Normal render pipelines (one per atlas, targeting `rgba8unorm` normal buffer)
- Normal buffer cleared to `(0.502, 0.502, 1.0, 1.0)` = flat normal (0,0,1) as default
- Lighting shader (`lighting.wgsl`) samples both scene color and normal buffer
- `N·L` dot product with simulated 3D light direction: `(delta.xy, light.radius * 0.3)`

### Alternatives Considered
- **Runtime Sobel compute shader**: Generates normals from scene color each frame. Higher runtime cost but works for procedural content. Deferred to future iteration.
- **Forward normal mapping**: Per-sprite normal sampling in the main fragment shader. Requires MRT and significant sprite pipeline changes.
- **Screen-space normals from depth**: No depth buffer in 2D; doesn't apply.

### Consequences
- **Pro:** Offline normal maps are zero runtime cost — just texture lookups
- **Pro:** Optional per-atlas — games without normal maps see no change
- **Pro:** Deferred buffer keeps sprite pipeline unchanged (no MRT complexity)
- **Pro:** Flat normal fallback ensures backward compatibility
- **Con:** Extra render pass and screen-sized `rgba8unorm` buffer (~8MB at 1080p) when normal maps active
- **Con:** Semi-transparent sprites get blended normals (acceptable for 2D)

---

## ADR-022: ZapZap Mini Example Game

**Date:** 2026-02-06
**Status:** Accepted

### Context
Phases 8-10 added render layers, dynamic lighting, and normal maps. We needed an example game that showcases all these features together. The ZapEngine project was originally inspired by the ZapZap circuit puzzle game, making a simplified port the natural choice.

### Decision
Created `examples/zapzap-mini/` — an 8x8 simplified version of the ZapZap circuit puzzle, ported from the native Rust crate at `/zapzap-native/crates/zapzap/`.

**Core mechanics ported:**
- 4-bit connection bitmask tiles (RIGHT=1, UP=2, LEFT=4, DOWN=8)
- `GRID_CODEP[16]` lookup table for atlas column mapping
- Two-pass BFS flood-fill: right edge first (Marking::Right), then left edge (→ Marking::Ok on overlap)
- Column-wise gravity after zap: surviving tiles shift down, new random tiles fill from top
- Rotation animation (0.2s lerp) and gravity-based fall animation

**Rendering features showcased:**
- Render layers: Background(0) for dark board, Terrain(1) for tiles + pins, Objects(2) for falling tiles, VFX(4) for arcs
- Dynamic point lights at marked tile positions: blue-white for Ok, indigo for Left, orange for Right
- Per-frame wiggle offset on Ok lights simulates arc-light flicker
- Low ambient (0.15, 0.15, 0.2) for dramatic bump-shadow effect via normal maps
- Electric arcs via engine's `add_arc()` with SkyBlue/Indigo/Orange/Red colors

**Simplified from native (no bonuses, no bot, no multiplayer, no power-ups):**
- Single player, endless mode with running score
- 8x8 board (native was 12x10)
- `GamePhase` enum: WaitingForInput, RotatingTile, FallingTiles, FreezeDuringZap

### Consequences
- **Pro:** Demonstrates dynamic lighting + normal maps in a real game context
- **Pro:** Validates the engine API with a non-trivial game (BFS, state machine, animations)
- **Pro:** No physics dependency — shows engine works well without rapier2d
- **Pro:** Reuses native-proven game logic (BFS, gravity, tile generation)
- **Con:** Rebuilds entire scene every frame (simple but not optimal for 80+ entities)

---

## ADR-023: TypeScript Engine as @zap/web NPM Package

**Date:** 2026-02-07
**Status:** Accepted

### Context
The TypeScript engine runtime (renderer, worker, assets, audio, React hook) lived at `src/engine/` and was consumed via Vite path aliases (`@zap/engine`). This worked within the monorepo but made it difficult to create new games with clean imports.

### Decision
Move the TypeScript engine to `packages/zap-web/` as a proper NPM package (`@zap/web`). Two entry points:
- `@zap/web` — core engine (renderer, assets, audio, protocol, `createEngineWorker()`)
- `@zap/web/react` — React hook (`useZapEngine`, types)

The package uses source-level exports (`.ts` files in the `exports` field) — no build step needed since Vite resolves them directly. Root `vite.config.ts` and `tsconfig.json` aliases point to the package source.

### Consequences
- **Pro:** Clean import paths (`@zap/web/react` instead of `../../src/engine/react`)
- **Pro:** `createEngineWorker()` factory enables non-React consumers
- **Pro:** Package boundary enforces separation between engine and game code
- **Pro:** Ready for future npm publishing if desired
- **Con:** Requires Vite aliases in root config (or a build step) for consumers

---

## ADR-024: `export_game!` Macro

**Date:** 2026-02-09
**Status:** Accepted

### Context
Each game crate contained ~244 lines of boilerplate in `lib.rs`: a `thread_local!` RUNNER storage, a `with_runner()` helper, and 40+ `#[wasm_bindgen]` exports for lifecycle functions, buffer pointers, capacities, SDF accessors, vector accessors, layer batches, bake state, and lighting state. This was duplicated across all 8 example games.

### Decision
Create an `export_game!` macro in `zap-web/src/lib.rs` that generates all boilerplate from a single invocation:

```rust
// Before: ~244 lines of boilerplate
// After:
zap_web::export_game!(MyGame, "my-game", vectors);
```

**Key design choices:**

1. **`macro_rules!` over proc-macro**: Simpler to implement, no separate crate needed, attributes like `#[wasm_bindgen]` expand correctly at the call site.

2. **Two variants**: Base variant generates core exports. Adding `, vectors` enables vector accessor exports (guarded by the `vectors` feature).

3. **`new()` convention**: The macro expects `$game_type::new()` to construct the game. Games don't implement `Default`; they explicitly define their constructor.

4. **Feature-gated exports**: Vector exports only generated when the `vectors` feature is enabled and the `, vectors` variant is used.

### Consequences
- **Pro:** 8 games reduced from ~1,952 lines total to ~64 lines total (97% reduction)
- **Pro:** New games get correct exports automatically — no copy-paste errors
- **Pro:** Protocol changes only require updating the macro, not all games
- **Pro:** `macro_rules!` is simpler than proc-macro and compiles faster
- **Con:** Error messages from macro expansion can be harder to debug
- **Con:** IDE support (go-to-definition) may not work inside macro-generated code

---

## ADR-025: Effects Module Decomposition

**Date:** 2026-02-09
**Status:** Accepted

### Context
`systems/effects.rs` had grown to 570 lines containing RNG, color lookup, geometry generation, electric arcs, particles, debug lines, and the `EffectsState` facade. The file was hard to navigate and had unrelated concerns mixed together.

### Decision
Split `effects.rs` into a `systems/effects/` directory with 7 focused submodules:

| File | Responsibility | Lines |
|------|----------------|-------|
| `rng.rs` | Xorshift64 RNG | ~30 |
| `segment_color.rs` | Arc color enum + UV lookup | ~80 |
| `geometry.rs` | Strip vertex generation | ~100 |
| `electric_arc.rs` | Midpoint-displacement arcs | ~80 |
| `particle.rs` | Particle struct + physics | ~65 |
| `debug_line.rs` | Debug line strip | ~10 |
| `mod.rs` | `EffectsState` facade + re-exports | ~160 |

Tests moved to their respective submodule files.

### Consequences
- **Pro:** Each file has a single responsibility (SRP)
- **Pro:** Easier to find and modify specific behavior
- **Pro:** Tests live next to the code they test
- **Pro:** No API changes — `lib.rs` imports unchanged
- **Con:** More files to navigate (7 vs 1)
- **Con:** Submodule re-exports add a small amount of boilerplate

---

## ADR-026: GameConfig-Driven Initialization

**Date:** 2026-02-09
**Status:** Accepted

### Context
Subsystems (`Scene`, `EffectsState`, `VectorState`, `LightState`) were initialized with hardcoded default capacities. `GameConfig` contained capacity settings but they weren't flowing to all subsystems.

### Decision
Add `with_capacity()` constructors to all subsystems and wire them through `EngineContext::with_config()`:

**New `GameConfig` fields:**
- `max_entities: usize` (default 512)
- `effects_seed: u64` (default 42)

**New constructors:**
- `Scene::with_capacity(cap)`
- `VectorState::with_capacity(max_vertices)`
- `LightState::with_capacity(max_lights)`
- `EffectsState::with_capacity(seed, max_vertices)`

**`EngineContext::with_config(&GameConfig)`** wires all capacities:
```rust
Self {
    scene: Scene::with_capacity(config.max_entities),
    effects: EffectsState::with_capacity(config.effects_seed, config.max_effects_vertices),
    lights: LightState::with_capacity(config.max_lights),
    vectors: VectorState::with_capacity(config.max_vector_vertices),
    // ...
}
```

### Consequences
- **Pro:** Single source of truth — `GameConfig` controls all allocations
- **Pro:** Games can tune memory usage (puzzle: small buffers, bullet-hell: large)
- **Pro:** Deterministic RNG seed for reproducible effects
- **Pro:** Backward compatible — `EngineContext::new()` still works with defaults
- **Con:** Config struct grows as new subsystems are added

---

## ADR-027: EngineContext Reorganization

**Date:** 2026-02-09
**Status:** Accepted

### Context
`EngineContext` was a large struct mixing fields and methods without clear organization. Bake state was spread across two fields (`baked_layers: u8`, `bake_generation: u32`). Camera was only used internally, not exposed to games.

### Decision
Reorganize `EngineContext` with three improvements:

**1. Extract `BakeState` struct:**
```rust
#[derive(Debug, Clone, Default)]
pub struct BakeState {
    mask: u8,
    generation: u32,
}

impl BakeState {
    pub fn bake(&mut self, layer: RenderLayer);
    pub fn invalidate(&mut self, layer: RenderLayer);
    pub fn unbake(&mut self, layer: RenderLayer);
    pub fn mask(&self) -> u8;
    pub fn generation(&self) -> u32;
    pub fn encode(&self) -> f32;
}
```

**2. Add public `Camera2D` field:**
```rust
pub camera: Camera2D  // Initialized from GameConfig world dimensions
```

**3. Organize with section comments:**
```rust
pub struct EngineContext {
    // Core state
    pub scene: Scene,
    pub effects: EffectsState,
    pub sounds: Vec<SoundEvent>,
    pub events: Vec<GameEvent>,

    // Rendering state
    pub camera: Camera2D,
    pub lights: LightState,
    pub bake: BakeState,

    // Optional systems
    #[cfg(feature = "vectors")]
    pub vectors: VectorState,
    #[cfg(feature = "physics")]
    pub physics: PhysicsWorld,

    // Private state
    next_id: u32,
    sprite_registry: SpriteRegistry,
    // ...
}
```

**Kept facade pattern:** `EngineContext` remains a facade because methods like `spawn_with_body()` and `despawn()` coordinate multiple subsystems atomically. Splitting into separate context types would fragment the API.

### Consequences
- **Pro:** `BakeState` is reusable and testable in isolation
- **Pro:** Games can access camera for pan/zoom (future integration)
- **Pro:** Code organization is clearer with section comments
- **Pro:** Backward compatible — existing methods still work
- **Con:** One more type to understand (`BakeState`)

---

## ADR-028: SAB Frame Reader Extraction

**Date:** 2026-02-09
**Status:** Accepted

### Context
SharedArrayBuffer parsing logic was embedded in `useZapEngine.ts` (~100 lines inside `drawFromBuffer()`). This made it:
1. Hard to test in isolation
2. Unavailable to non-React consumers (vanilla TS, Svelte)
3. Mixed with rendering concerns

### Decision
Extract SAB parsing into a standalone utility in `worker/frame-reader.ts`:

```typescript
export interface FrameState {
  instanceData: Float32Array;
  instanceCount: number;
  atlasSplit: number;
  effectsData?: Float32Array;
  effectsVertexCount: number;
  sdfData?: Float32Array;
  sdfInstanceCount: number;
  vectorData?: Float32Array;
  vectorVertexCount: number;
  layerBatches?: LayerBatchDescriptor[];
  bakeState?: BakeState;
  lightingState?: LightingState;
}

export function readFrameState(
  buf: Float32Array,
  layout: ProtocolLayout
): FrameState | null;
```

**Key design choice:** Reads stay in the UI thread. The whole point of SharedArrayBuffer is zero-copy access. Moving parsing to a worker would add `postMessage` latency and defeat the purpose.

### Consequences
- **Pro:** SAB parsing is testable in isolation
- **Pro:** Non-React consumers can import `readFrameState` directly
- **Pro:** `useZapEngine.ts` is simpler — focuses on React lifecycle
- **Pro:** Type-safe `FrameState` interface documents the frame contract
- **Con:** One more module to maintain
- **Con:** Import path is longer (`@zap/web` vs direct file)
