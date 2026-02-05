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


* **Task 2.3: Debug Rendering (Optional but Recommended)**
* Implement a simple line-drawer to visualize colliders (hitboxes) for debugging.



---

### **Phase 3: Visuals & Chemistry (The "Zap" Layer)** DONE

**Goal:** Enable HDR glow and "Fake 3D" spheres for educational content.

* **Task 3.1: Generic Particle System**
* Refactor `effects.rs` from ZapZap.
* Create a configurable `Emitter` component (Rate, Lifetime, Color Gradient, Drag).
* Ensure the engine automatically handles particle lifecycle and writes to the `effects_buffer`.


* **Task 3.2: The "Molecule" Pipeline (SDF)**
*
* **Shader:** Write `molecule.wgsl` using Raymarching (SDF) to draw perfect spheres on 2D quads.
* **Rust:** Add a `Mesh` component to `Entity`.
* **Renderer:** Update `webgpu.ts` to handle a third render pass: `customPipeline`.

MUST PROVIDE FALLBACK:

WebGPU + HDR/EDR (rgba16float, display-p3, extended tone mapping)
  |  fails? (toneMapping unsupported)
  v
WebGPU + sRGB (rgba16float, no HDR features)
  |  fails? (rgba16float unsupported)
  v
WebGPU + preferred format (bgra8unorm, basic sRGB)
  |  fails? (WebGPU unavailable entirely)
  v
Canvas 2D (software rendering, SDR only)

you can still get inspiration from /Users/apple/Documents/Xcodes/ZapZap/zapzap-native/

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

* Created `src/engine/react/useZapEngine.ts` — React hook encapsulating worker lifecycle, renderer init (WebGPU→Canvas2D fallback), SAB reading, rAF render loop, input forwarding, resize, audio, and game events.
* API: `useZapEngine({ wasmUrl, assetsUrl })` → `{ canvasRef, sendEvent, fps, isReady, canvasKey }`.
* Separate import path: `@zap/engine/react` (core engine stays React-free).

### Task 4.3: React Demo ✓

* Created `examples/react-demo/` with `App.tsx` showing the hook in action with an FPS HUD overlay.
* Reuses basic-demo WASM + assets (no new Rust code).

### Task 4.4: (Future) The Template Repository

* A `zap-engine-template` repository with `infra/` (CDK) pre-configured and a "Hello World" Rust file.
* Deferred — best done as a separate repository once the engine is published.

