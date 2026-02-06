# ZapEngine

**The Ferrari Engine inside a React Chassis.**

ZapEngine is a headless, data-oriented 2D game engine for the modern web. Game simulation runs in **Rust/WASM** inside a Web Worker, rendering uses **WebGPU** (with Canvas2D fallback), and your UI stays in **React**. Zero jank, 60FPS+.

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) with `wasm32-unknown-unknown` target
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [Node.js](https://nodejs.org/) (v18+)

```bash
# Install wasm target if you haven't
rustup target add wasm32-unknown-unknown

# Install wasm-pack if you haven't
cargo install wasm-pack

# Install JS dependencies
npm install
```

### Build WASM for an example

```bash
# Basic Demo
wasm-pack build examples/basic-demo --target web --out-dir pkg

# Physics Playground (Angry Birds clone)
wasm-pack build examples/physics-playground --target web --out-dir pkg

# Chemistry Lab (SDF molecules)
wasm-pack build examples/chemistry-lab --target web --out-dir pkg

# ZapZap Mini (circuit puzzle with dynamic lighting)
wasm-pack build examples/zapzap-mini --target web --out-dir pkg
```

### Run the dev server

```bash
npm run dev
```

This starts Vite at `http://localhost:5173` with the required COOP/COEP headers for SharedArrayBuffer.

### Open an example

- **Basic Demo:** http://localhost:5173/
- **Physics Playground:** http://localhost:5173/examples/physics-playground/index.html
- **Chemistry Lab:** http://localhost:5173/examples/chemistry-lab/index.html
- **ZapZap Mini:** http://localhost:5173/examples/zapzap-mini/index.html

## Architecture

```
zap-engine/
├── crates/
│   ├── zap-engine/          # Core engine library (pure Rust, no WASM deps)
│   │   └── src/
│   │       ├── api/         # Game trait, EngineContext, GameConfig
│   │       ├── components/  # Entity, Sprite, Layer, Emitter, Mesh
│   │       ├── core/        # Scene, Physics (rapier2d), FixedTimestep
│   │       ├── systems/     # Effects, Rendering, Lighting, Text, Vectors
│   │       ├── renderer/    # RenderInstance, Camera2D, SDF buffers
│   │       ├── bridge/      # SharedArrayBuffer protocol layout
│   │       ├── input/       # InputQueue, InputEvent
│   │       └── assets/      # Manifest parser, SpriteRegistry
│   │
│   └── zap-web/             # WASM bridge (GameRunner, wasm-bindgen glue)
│
├── src/engine/              # TypeScript runtime
│   ├── renderer/            # WebGPU + Canvas2D renderers, shaders
│   ├── worker/              # Web Worker (loads WASM, runs game loop)
│   ├── react/               # useZapEngine hook
│   ├── assets/              # Manifest loader, normal map loader
│   └── audio/               # SoundManager
│
├── examples/
│   ├── basic-demo/          # Minimal spinning sprite
│   ├── zap-engine-template/ # Starter template for new games
│   ├── physics-playground/  # Angry Birds-style sling + tower
│   ├── chemistry-lab/       # SDF molecule visualization
│   └── zapzap-mini/         # Circuit puzzle (lighting + normal maps)
│
├── tools/
│   ├── bake-assets.ts       # Asset manifest generator
│   ├── generate_normals.py  # Normal map generator (Sobel operator)
│   └── font-atlas-generator.html  # Browser-based font atlas tool
│
└── docs/
    ├── VISION.md            # Project manifesto and design goals
    └── DECISIONS.md         # Architectural Decision Records (ADR-001 to ADR-022)
```

## How It Works

```
┌─────────────────────────────────────────────────────────┐
│  Main Thread                                            │
│  ┌──────────┐  ┌──────────┐  ┌────────────────────────┐│
│  │  React   │  │  Audio   │  │  WebGPU / Canvas2D     ││
│  │   UI     │  │ Manager  │  │  Renderer              ││
│  └────┬─────┘  └────┬─────┘  └────────┬───────────────┘│
│       │              │                 │  reads          │
│       │    ┌─────────┴─────────────────┤                │
│       │    │   SharedArrayBuffer       │                │
│       │    │  ┌─────┬────────┬───────┐ │                │
│       │    │  │Head │Sprites │Effects│ │                │
│       │    │  │er   │Lights  │Sounds │ │                │
│       │    │  └─────┴────────┴───────┘ │                │
│       │    └─────────┬─────────────────┘                │
│       │              │  writes                          │
│  ─────┼──────────────┼──────────────────────────────────│
│  Web Worker          │                                  │
│  ┌───────────────────┴────────────────────────────────┐ │
│  │  Rust / WASM                                       │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐ │ │
│  │  │  Game    │  │ Physics  │  │ Render Buffer    │ │ │
│  │  │  Logic   │  │ (Rapier) │  │ Builder          │ │ │
│  │  └──────────┘  └──────────┘  └──────────────────┘ │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

1. **Game logic** runs in Rust/WASM inside a Web Worker at a fixed 60Hz timestep
2. **Physics** (rapier2d) steps in the worker, positions sync to entities automatically
3. **Render data** is written to a SharedArrayBuffer — zero-copy, zero-serialization
4. **Main thread** reads the SAB each frame and draws via WebGPU (or Canvas2D fallback)
5. **React** stays on the main thread for UI overlays, buttons, HUD — no canvas hacks

## Creating a New Game

1. Copy the template:
   ```bash
   cp -r examples/zap-engine-template examples/my-game
   ```

2. Update `examples/my-game/Cargo.toml` — change the package name

3. Add to workspace in root `Cargo.toml`:
   ```toml
   members = [
       # ...
       "examples/my-game",
   ]
   ```

4. Implement the `Game` trait in Rust:
   ```rust
   impl Game for MyGame {
       fn config(&self) -> GameConfig {
           GameConfig {
               world_width: 800.0,
               world_height: 600.0,
               ..GameConfig::default()
           }
       }

       fn init(&mut self, ctx: &mut EngineContext) {
           // Spawn entities, set up the scene
       }

       fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
           // Game logic, handle input, spawn/despawn
       }
   }
   ```

5. Build and run:
   ```bash
   wasm-pack build examples/my-game --target web --out-dir pkg
   npm run dev
   ```

## Engine Features

| Feature | Description |
|---------|-------------|
| **Sprites** | Texture atlas rendering with per-instance transforms, alpha, blend modes |
| **Physics** | rapier2d integration — rigid bodies, colliders, joints (fixed, spring, revolute) |
| **Effects** | Electric arcs (midpoint displacement), particle system with attractors |
| **SDF Rendering** | Raymarched spheres, capsules, rounded boxes with Phong + Fresnel shading |
| **Vectors** | CPU-tessellated polygons, rectangles, circles, polylines via lyon |
| **Dynamic Lighting** | Point lights with quadratic falloff, per-layer masking, ambient control |
| **Normal Maps** | Offline Sobel generation, deferred normal buffer, N*L directional shading |
| **Render Layers** | 6 layers (Background through UI), per-layer baking for static content |
| **Text** | Font atlas system — spawn text as sprite entities |
| **HDR** | Tier-aware rendering: HDR-EDR, HDR-sRGB, SDR, Canvas2D fallback |
| **Audio** | Sound events from Rust, played on main thread via Web Audio API |
| **React Integration** | `useZapEngine` hook handles worker, canvas, input, resize, audio |
| **Custom Events** | Bidirectional: React sends `CustomEvent` to Rust, Rust emits `GameEvent` to React |

## Example Games

### Physics Playground
Angry Birds-style physics sandbox. Drag to aim a sling, launch projectiles at a tower of blocks. Demonstrates sprites, rapier2d physics, collision detection, and custom events.

### Chemistry Lab
Interactive molecule builder using SDF raymarching. Atoms are rendered as mathematically perfect spheres with Fresnel reflections. Spring joints simulate molecular bonds. No sprite textures needed.

### ZapZap Mini
8x8 circuit puzzle ported from the native ZapZap game. Tap tiles to rotate their connections. When a path forms from left to right, electric arcs light up with dynamic point lights that cast bump-mapped shadows on the tile board via normal maps.

## Scripts

```bash
npm run dev          # Start Vite dev server
npm run build        # Production build
npm run check        # TypeScript type check
npm run bake-assets  # Generate asset manifests
```

## Tests

```bash
# Run all engine tests (111 tests)
cargo test --manifest-path crates/zap-engine/Cargo.toml

# Run a specific example's tests
cargo test -p zapzap-mini

# Check WASM compilation
cargo check --target wasm32-unknown-unknown -p zapzap-mini
```

## Documentation

- [VISION.md](docs/VISION.md) — Project manifesto and design goals
- [DECISIONS.md](docs/DECISIONS.md) — Architectural Decision Records (22 ADRs)
- Every directory has a `MAP.md` explaining its purpose and connections

## License

Private project.
