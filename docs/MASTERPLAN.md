where are we on the grand plan?

### **Phase 1: The Great Extraction (Refactoring)** DONE - and with SAB parametrization

**Goal:** Decouple the "Ferrari Engine" from the "ZapZap" game logic so it can run *any* game.

* **Task 1.1: Initialize the Workspace**
* Create a cargo workspace root.
* Create the crate structure: `crates/zap-engine` (Library), `crates/zap-web` (Bridge), and `examples/basic-demo`.
* **Action:** Move `crates/engine` code into `crates/zap-engine`.


* **Task 1.2: Define the `Game` Trait**
* In `zap-engine/src/lib.rs`, define the contract that decouples logic from the loop.
* *Requirement:* Methods for `init`, `update` (fixed timestep), and `render` (interpolation support).


* **Task 1.3: The "Fat Entity" Struct**
* Create `src/scene.rs` in the engine crate.
* Define a single `Entity` struct containing optional components: `Position`, `Sprite`, `PhysicsBody` (placeholder), `Emitter` (placeholder).
* *Constraint:* Ensure memory layout is compatible with `SharedArrayBuffer` (use `repr(C)`).


* **Task 1.4: Abstract the Sprite Renderer**
* Remove hardcoded constants (Atlas Rows/Cols) from the shader and Rust code.
* **Action:** Pass texture dimensions via Uniforms or Instance data.
* Implement `AssetManifest` loading (JSON) to map string IDs ("hero") to UV coordinates.


* **Task 1.5: Generic Web Worker**
* Rewrite `sim.worker.ts` to be game-agnostic.
* It should accept a configuration object (WASM URL, Asset URL) at startup.



---

### **Phase 2: The Physics Integration (Angry Birds Layer)** ADDED RAPIER2D

**Goal:** Add industrial-grade 2D physics without ruining performance.

* **Task 2.1: Integrate Rapier2D**
* Add `rapier2d` to `zap-engine` dependencies.
* Initialize `PhysicsPipeline`, `IslandManager`, `BroadPhase`, and `NarrowPhase` in the `EngineContext`.


* **Task 2.2: The Physics-Render Sync**
* Create a system that runs *after* the physics step but *before* the render snapshot.
* **Logic:** Iterate all active rigid bodies  Copy X/Y/Rotation to the `RenderInstance` buffer.
* *Constraint:* Physics determines position. Game logic applies forces, not coordinates.


* **Task 2.3: Debug Rendering** ✓
* Implemented `debug_draw_colliders()` — reuses the effects pipeline for zero-cost debug visualization.
* `collider_shape()` on PhysicsWorld extracts shape info; outlines for Ball (24-seg circle), Cuboid (rotated rect), CapsuleY (semicircles+sides).
* `DebugLine` type in EffectsState, included in `rebuild_effects_buffer()`. Opt-in per frame from `Game::update()`.



---

### **Phase 3: Visuals & Chemistry (The "Zap" Layer)** DONE

**Goal:** Enable HDR glow and "Fake 3D" spheres for educational content.

* **Task 3.1: Generic Particle System** ✓
* Refactored `effects.rs` from ZapZap.
* Created a configurable `EmitterComponent` (Rate, Lifetime, Color Gradient, Drag) with continuous and burst emission modes.
* Per-particle physics fields (drag, attract_strength, speed_factor). `tick_emitters()` auto-spawns from entities.

* **Task 3.2: The "Molecule" Pipeline (SDF)** ✓
* `molecule.wgsl` — raymarched spheres with Phong + Fresnel + HDR emissive.
* `MeshComponent` on Entity, `SDFInstance` buffer (12 floats/48 bytes).
* WebGPU: separate pipeline with storage buffer. Canvas2D: radial gradient circles.
* Draw order: sprites → SDF → effects.

* **Task 3.3: Tier-Aware HDR Fallback Chain** ✓
* 4-tier rendering cascade: hdr-edr → hdr-srgb → sdr → canvas2d.
* WGSL override constants (`EFFECTS_HDR_MULT`, `SDF_EMISSIVE_MULT`) set per tier at pipeline creation.
* `RenderTier` type exposed on Renderer interface. Per-tier glow multipliers:
  - hdr-edr: 6.4 / 5.4, hdr-srgb: 3.0 / 2.5, sdr: 1.0 / 0.5.
* Resize reconfigures canvas based on negotiated tier.

---

### **Phase 4: The Developer Experience (DX)** DONE

**Goal:** Make it usable without knowing WGPU internals.

### Task 4.0: Fix basic-demo SDF passthrough ✓
* Updated `examples/basic-demo/main.ts` to pass SDF data through to the renderer.

### Task 4.1: The Asset Baker ✓

* Created `tools/bake-assets.ts` — convention-based CLI that scans image folders and outputs `assets.json`.
* Naming convention: `hero_4x8.png` → atlas with 4 cols, 8 rows. Plain files → 1×1 single-sprite atlas.
* Run via `npm run bake-assets <input-dir> [--output assets.json]`.

### Task 4.2: The `useZapEngine` Hook ✓

* Created `useZapEngine.ts` React hook (now at `packages/zap-web/src/react/useZapEngine.ts`) — encapsulates worker lifecycle, renderer init (WebGPU→Canvas2D fallback), SAB reading, rAF render loop, input forwarding, resize, audio, and game events.
* API: `useZapEngine({ wasmUrl, assetsUrl })` → `{ canvasRef, sendEvent, fps, isReady, canvasKey }`.
* Separate import path: `@zap/web/react` (core engine stays React-free).

### Task 4.3: React Demo ✓

* Created `examples/react-demo/` with `App.tsx` showing the hook in action with an FPS HUD overlay.
* Reuses basic-demo WASM + assets (no new Rust code).

### Task 4.4: Game Template ✓

* Created `examples/zap-engine-template/` — minimal starter skeleton for new games.
* Renders a spinning sprite with the minimum boilerplate: `Game` trait impl, wasm-bindgen exports, TypeScript entry, HTML shell.
* Copy the directory, rename, and start building. See its `README.md` for quick-start instructions.

---

### **Phase 5: Audio Completion + Extended SDF Shapes** DONE

**Goal:** Complete the manifest-driven audio pipeline and add Capsule/RoundedBox SDF shapes for chemistry molecule rendering.

### Task 5.1: Audio System Completion ✓

* **Per-sound volume**: `SoundEntry` interface (`{ path, volume? }`) on `SoundConfig.sounds`. Playback routes through `GainNode` when `volume < 1.0`. Backward compatible — plain strings still work.
* **Manifest bridge**: `buildSoundConfigFromManifest(manifest, basePath)` utility converts `AssetManifest.sounds` entries (with `event_id`) to `SoundConfig`. Exported from `@zap/web`.
* **Eager init**: `SoundManager.init()` called immediately in `useZapEngine` hook. AudioContext starts suspended; `resume()` on pointerdown handles unsuspension. Pre-decodes buffers for zero first-play latency.
* **Manifest sounds section**: Added `"sounds": {}` to basic-demo and template `assets.json` manifests.

### Task 5.2: Extended SDF Shapes ✓

* **New shapes**: `SDFShape::Capsule { radius, half_height }` (bonds) and `SDFShape::RoundedBox { radius, half_height, corner_radius }` (labels).
* **Zero protocol change**: Repurposed 3 padding fields in `SDFInstance` — `_pad0 → shape_type`, `_pad1 → half_height`, `_pad2 → extra`. 48 bytes / 12 floats preserved. Zeroed padding = Sphere (backward compatible).
* **Convenience builders**: `MeshComponent::sphere()`, `::capsule()`, `::rounded_box()`.
* **Shader dispatch** (`molecule.wgsl`): SDF primitives (`sdf_sphere`, `sdf_capsule`, `sdf_rounded_box`), float-threshold shape dispatch, entity rotation applied to quads, central-difference normals for non-sphere shapes, same Phong + Fresnel + HDR pipeline for all shapes.
* **Canvas 2D fallback**: Capsule/RoundedBox drawn as rotated rounded rectangles with linear gradients.
* **8 new tests** (62 total): mesh builders (3), SDFInstance encoding (2), build_sdf_buffer multi-shape (2), manifest sound parsing (1).

---

### **Phase 6: Gameplay Quality of Life + Example Games** DONE

**Goal:** Make the engine usable for real games by adding missing engine features, then prove it works with 2 complete example games.

#### Part A: Engine Enhancements

### Task 6.A1: Sprite Name Lookup from Manifest ✓

* `SpriteRegistry` in `crates/zap-engine/src/assets/registry.rs` — converts manifest sprite descriptors to `SpriteComponent` for name-based lookup.
* `EngineContext::load_manifest(json)` populates the registry; `ctx.sprite("hero")` returns a ready-to-use `SpriteComponent`.
* Manifest JSON passed from TypeScript → Worker → WASM during init via `game_load_manifest` export.
* 3 new tests: registry load, unknown returns None, EngineContext sprite lookup.

### Task 6.A2: World Coordinate Conversion ✓

* Worker-side `screenToWorld()` converts canvas CSS pixel coordinates to world coordinates using `computeProjection()`.
* React hook sends canvas CSS dimensions via `resize` message on every ResizeObserver callback.
* All pointer events (down, up, move) now arrive in world coordinates — games no longer need manual conversion.

### Task 6.A3: Joints API ✓

* `JointDesc` enum: `Fixed`, `Spring { rest_length, stiffness, damping }`, `Revolute` — all with local anchors.
* `JointHandle` wraps Rapier's `ImpulseJointHandle`.
* `PhysicsWorld::create_joint()`, `remove_joint()`, `joint_count()`.
* `EngineContext::create_joint(entity_a, entity_b, desc)` convenience (looks up physics bodies).
* 4 new tests: create/remove, fixed constraint, spring force, revolute pendulum.

### Task 6.A4: Custom Events (React → Rust) ✓

* `InputEvent::Custom { kind: u32, a: f32, b: f32, c: f32 }` variant for UI-driven game events.
* Worker `'custom'` message handler forwards to `game_custom_event` WASM export.
* All game crates export `game_custom_event(kind, a, b, c)`.
* 1 new test: custom event push and drain.

#### Part B: Example Games

### Task 6.B1: Chemistry Lab ✓

* Interactive molecule builder — click to place atoms (SDF spheres), drag between atoms to create bonds (SDF capsules + spring joints).
* Elements: H (white, 1 bond), O (red, 2), C (gray, 4), N (blue, 3).
* React UI: element selector buttons, clear, atom/bond count display.
* Demonstrates: SDF rendering, physics joints, custom events, world coordinates.

### Task 6.B2: Physics Playground ✓

* Angry Birds-style sandbox — drag sling to launch projectile at block tower.
* 5×3 tower of cuboid blocks, sling at left, collision sparks via particle effects.
* Game states: Aiming → Flying → Settled. Score = knocked blocks.
* React UI: score display, reset button, FPS counter.
* Demonstrates: sprites + physics, collision events, custom events, world coordinates.

**8 new tests** (70 total): registry (2), sprite lookup (1), custom event (1), joints (4).
