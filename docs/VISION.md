# Vision: ZapEngine

## 1. The Manifesto

**"The Ferrari Engine inside a React Chassis."**

ZapEngine is a headless, data-oriented game engine designed for the modern web. It solves the "JavaScript Stutter" problem by moving 100% of the game simulation and rendering command generation into **Rust/WASM**, leaving the Main Thread free for React UI.

It is built for **Educational & Casual High-Performance Games**:

* **Performance:** 60FPS+ on low-end hardware (e.g., Raspberry Pi 5).
* **Visuals:** WebGPU-native with HDR glow and SDF-based pseudo-3D.
* **Physics:** Industrial-grade 2D physics (via Rapier).
* **DX:** React is a first-class citizen. UI is HTML/CSS, not a canvas overlay.

---

## 2. Core Pillars

### I. The "Headless" Philosophy

The engine does not own the window. It does not own the DOM. It owns a `OffscreenCanvas` and a `SharedArrayBuffer`.

* **Host (TypeScript):** Handles Inputs, Audio, and React UI.
* **Guest (Rust):** Handles Physics, Logic, and Render Batching.
* **Bridge:** Zero-copy synchronization via Atomics.

### II. The "Fat Entity" System

To keep the API simple for rapid prototyping (and educational use), we eschew complex ECS (Entity Component System) boilerplate in favor of a "Fat Entity" model. A single struct holds optional components for Physics, Sprites, and Effects.

### III. Hybrid Rendering

We support three distinct visual paradigms in a single render pass:

1. **Sprites:** Standard 2D textured quads (Game Characters, UI).
2. **VFX:** Additive blending for HDR glows (Fire, Magic, Lasers).
3. **SDF Meshes:** Raymarched signed distance fields for mathematically perfect "3D" shapes (Atoms, Planets) without the overhead of a 3D engine.

---

## 3. Technical Requirements

### Physics Module ("The Angry Birds Layer")

* **Backend:** `rapier2d` (Rust).
* **Features:** Rigid Bodies, Colliders, Gravity, Joints.
* **Integration:** Physics steps happen in the Worker. The engine automatically syncs Rapier positions to the Render Buffer.

### VFX Module ("The ZapZap Layer")

* **Particle System:** CPU-simulated, GPU-instanced.
* **emitters:** Configurable bursts (fireworks), streams (smoke), and trails.
* **HDR:** 16-bit Float textures with Bloom/Glow support on capable hardware.

### Chemistry Module ("The SDF Layer")

* **Goal:** Render 3D molecules for educational games without loading 3D models.
* **Tech:** Custom WGSL Fragment Shader using **Raymarching**.
* **Input:** Position, Radius, Color.
* **Output:** A lit, shaded, 3D sphere rendered onto a 2D quad.

---

## 4. The API (Rust)

Developers implement the `Game` trait. The engine handles the loop, the worker, and the GPU.

```rust
/// The contract every game must fulfill
pub trait Game {
    /// Setup initial state, load assets, configure physics
    fn init(&mut self, ctx: &mut EngineContext);

    /// The Game Loop (Fixed Timestep)
    /// Apply forces, check win conditions, spawn entities
    fn update(&mut self, ctx: &mut EngineContext, dt: f32);

    /// Read-only render pass
    /// The engine automatically renders Entities, but you can 
    /// push manual commands here (e.g., debug lines)
    fn render(&self, ctx: &mut RenderContext);
}

/// The "Fat Entity" - easy to understand for beginners
pub struct Entity {
    pub pos: Vec2,
    pub sprite: Option<String>,        // "hero_idle"
    pub physics: Option<RigidBody>,    // Mass, Velocity, Collider
    pub effects: Option<Emitter>,      // "fire_trail"
    pub mesh: Option<SDFShape>,        // "atom_sphere"
}

```

---

## 5. Integration Guide

### How to use ZapEngine in a Project

The architecture is designed as a **Monorepo Workspace**.

#### A. Directory Structure

```text
my-educational-games/
├── infra/                  # CDK (S3 + CloudFront + COOP/COEP headers)
├── packages/
│   ├── zap-engine/         # The Core Rust Library (Physics + WGPU)
│   ├── zap-web/            # The TypeScript/React Bridge
│
├── games/
│   ├── chemistry-lab/      # Your React App + Rust Game Logic
│   │   ├── src/
│   │   │   ├── lib.rs      # impl Game for ChemistrySim
│   │   │   ├── App.tsx     # React UI
│   │   │   └── assets.json # Asset Manifest
│   │   └── Cargo.toml
│
│   └── physics-playground/ # Another Game
│       └── ...

```

#### B. The React Hook (`useZapEngine`)

We provide a hook that handles the Worker startup, Canvas transfer, and resizing.

```tsx
// games/chemistry-lab/src/App.tsx
import { useZapEngine } from '@zap/web';

export function App() {
  const { canvasRef, sendEvent, fps } = useZapEngine({
    wasmUrl: '/chemistry_bg.wasm', // The compiled Rust logic
    assets: '/assets.json'
  });

  return (
    <div className="game-container">
      {/* The Engine Output */}
      <canvas ref={canvasRef} />
      
      {/* The React UI Overlay */}
      <div className="hud">
        <h1>Molecule Builder</h1>
        <button onClick={() => sendEvent({ type: 'SPAWN_ATOM', element: 'H' })}>
          Add Hydrogen
        </button>
        <span className="fps">{fps} FPS</span>
      </div>
    </div>
  );
}

```

#### C. The Asset Pipeline

Instead of hardcoding atlas coordinates, the engine expects a standard `assets.json` generated by our script.

**Workflow:**

1. Drop `hydrogen.png`, `carbon.png` into `assets/`.
2. Run `npm run bake-assets`.
3. In Rust: `ctx.spawn_sprite("hydrogen", pos)`.

---

## 6. Development Roadmap

| Phase | Name | Focus | Deliverables |
| --- | --- | --- | --- |
| **1** | **Extraction** | Refactoring | Decouple `crates/engine` from ZapZap. Create `Game` trait. |
| **2** | **Physics** | Angry Birds | Integrate `rapier2d`. Sync Rigidbody  Renderer. |
| **3** | **SDF** | Chemistry | Implement `molecule.wgsl` raymarching shader. |
| **4** | **Packaging** | NPM/Cargo | Publish `useZapEngine` and `zap-engine` crate. |