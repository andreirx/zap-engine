# ZapEngine Architecture Schematics

Reference document for software architects. All diagrams derived from source-level inspection.

---

## 1. System-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              BROWSER PROCESS                                │
│                                                                             │
│  ┌─ Main Thread ──────────────────────────────┐  ┌─ Web Worker ────────────┐│
│  │                                            │  │                         ││
│  │  ┌──────────┐  ┌──────────┐  ┌───────────┐ │  │  ┌───────────────────┐  ││
│  │  │  React   │  │  Sound   │  │  WebGPU   │ │  │  │ engine.worker.ts  │  ││
│  │  │   App    │  │ Manager  │  │ Renderer  │ │  │  │                   │  ││
│  │  │          │  │ (WebAudio│  │ (or C2D   │ │  │  │ ┌───────────────┐ │  ││
│  │  │ useZap   │  │  API)    │  │  fallback)│ │  │  │ │  WASM Module  │ │  ││
│  │  │ Engine   │  │          │  │           │ │  │  │ │               │ │  ││
│  │  │ hook     │  │          │  │ frame-    │ │  │  │ │ GameRunner<G> │ │  ││
│  │  │          │  │          │  │ reader.ts │ │  │  │ │   Game trait  │ │  ││
│  │  └────┬─────┘  └────┬─────┘  └─────┬─────┘ │  │  │ │   rapier2d    │ │  ││
│  │       │             │              │       │  │  │ │   effects     │ │  ││
│  │       │             │       reads  │       │  │  │ │   scene/ent   │ │  ││
│  │       │             │              │       │  │  │ └───────┬───────┘ │  ││
│  │       │  ┌─────────────────────────┤       │  │  │         │         │  ││
│  │       │  │  SharedArrayBuffer      │       │  │  │         │writes   │  ││
│  │       │  │  (zero-copy shared mem) │◄──────┼──┼──┼─────────┘         │  ││
│  │       │  └─────────────────────────┘       │  │  │                   │  ││
│  │       │                                    │  │  └───────────────────┘  ││
│  │       │  postMessage (input, sound events, │  │                         ││
│  │       └──── game events, init/resize) ─────┼──┘                         ││
│  │                                            │                            ││
│  └────────────────────────────────────────────┘                            ││
└─────────────────────────────────────────────────────────────────────────────┘
```

**Data flow directionality:**

```
                    postMessage
 React UI ──────────────────────────►  Worker
 (input events,                        (forwards to WASM via
  custom events,                        wasm-bindgen FFI)
  resize)

                    postMessage
 SoundManager ◄────────────────────── Worker
 (sound event IDs,                     (reads from WASM heap,
  game events)                          sends via postMessage)

                SharedArrayBuffer
 Renderer ◄════════════════════════► Worker
 (reads instances, effects,            (writes every frame:
  SDF, vectors, lights,                 WASM heap → memcpy → SAB)
  layer batches, bake state)

 Synchronization: Worker writes SAB → Atomics.store(lock, 1) → Atomics.notify
                  Renderer reads in rAF loop, checks frame_counter for new data
```

---

## 2. Crate Dependency Graph

```
┌─────────────────────────────────────────────────────────┐
│  examples/physics-playground  (bin crate, wasm target)  │
│    Cargo.toml: zap-engine + zap-web + wasm-bindgen      │
│    src/lib.rs:  zap_web::export_game!(PhysicsPlayground)│
└───────────────────┬──────────────────┬──────────────────┘
                    │                  │
         ┌──────────▼──────┐  ┌────────▼──────────┐
         │  crates/zap-web │  │ crates/zap-engine │
         │  (WASM bridge)  │  │ (pure Rust lib)   │
         │                 │  │                   │
         │  GameRunner<G>  │  │  Game trait       │
         │  export_game!   │──│  EngineContext    │
         │  wasm-bindgen   │  │  Scene, Entity    │
         │  thread_local!  │  │  PhysicsWorld     │
         │                 │  │  EffectsState     │
         │  deps:          │  │  RenderBuffer     │
         │   wasm-bindgen  │  │  ProtocolLayout   │
         │   web-sys       │  │                   │
         │   console_log   │  │  deps:            │
         │   console_error │  │   glam 0.30       │
         └─────────────────┘  │   bytemuck 1.21   │
                              │   serde/json 1    │
                              │   rapier2d 0.22 ? │
                              │   lyon 1.0 ?      │
                              └───────────────────┘
                              ? = feature-gated optional
```

**Feature gates:**

```
zap-engine features:
  default = ["physics", "vectors"]
  physics = ["dep:rapier2d"]     →  PhysicsWorld, BodyDesc, ColliderDesc, Joints
  vectors = ["dep:lyon"]         →  VectorState, polygon tessellation

Compile-time effect:
  physics OFF  →  Entity.body removed, ctx.physics removed, ~500KB smaller WASM
  vectors OFF  →  VectorState removed, no get_vector_* exports
```

---

## 3. Rust Engine Internals (`crates/zap-engine`)

### 3.1 Module Map

```
crates/zap-engine/src/
│
├── lib.rs                  ── Re-exports public API
│
├── api/                    ── PUBLIC SURFACE
│   ├── game.rs             ── Game trait, GameConfig, EngineContext, BakeState, RenderContext
│   └── types.rs            ── EntityId(u32), SoundEvent(u8), GameEvent{kind,a,b,c}
│
├── components/             ── DATA TYPES (no behavior)
│   ├── entity.rs           ── Fat Entity struct (~200+ bytes)
│   ├── sprite.rs           ── SpriteComponent {atlas, col, row, cell_span, alpha, blend}
│   ├── emitter.rs          ── EmitterComponent (particle config)
│   ├── mesh.rs             ── MeshComponent + SDFShape enum (Sphere/Capsule/RoundedBox)
│   ├── layer.rs            ── RenderLayer enum (Background..UI, 6 variants)
│   ├── animation.rs        ── AnimationComponent (frame cycling)
│   └── tilemap.rs          ── TilemapComponent (grid-based level data)
│
├── core/                   ── STATEFUL SUBSYSTEMS
│   ├── scene.rs            ── Scene: Vec<Entity> storage, O(n) lookup
│   ├── physics.rs          ── PhysicsWorld: rapier2d wrapper, 9 internal structs
│   └── time.rs             ── FixedTimestep: accumulator-based fixed-dt loop
│
├── systems/                ── PER-FRAME LOGIC (stateless transforms on data)
│   ├── render.rs           ── build_render_buffer(): sort by (layer, atlas), pack f32s
│   ├── sdf_render.rs       ── build_sdf_buffer(): Entity.mesh → SDFInstance buffer
│   ├── effects/            ── EffectsState: particles, electric arcs, geometry builders
│   │   ├── mod.rs          ── EffectsState, spawn_particles, rebuild_effects_buffer
│   │   ├── particle.rs     ── Particle struct, physics sim (drag, attraction)
│   │   ├── electric_arc.rs ── Midpoint displacement algorithm, strip → triangle conversion
│   │   ├── geometry.rs     ── build_strip_vertices, strip_to_triangles
│   │   ├── segment_color.rs── 13-entry color palette for effect segments
│   │   ├── debug_line.rs   ── Debug line rendering via effects pipeline
│   │   └── rng.rs          ── Xoshiro256++ (deterministic, no std::rand dependency)
│   ├── emitter.rs          ── tick_emitters(): Entity.emitter → EffectsState.spawn_particles
│   ├── lighting.rs         ── LightState: Vec<PointLight>, persistent, SAB-serializable
│   ├── vector.rs           ── VectorState: CPU tessellation via lyon, RGBA vertex buffer
│   ├── text.rs             ── Font atlas → Entity spawning, build_text_entities()
│   ├── animation.rs        ── tick_animations(): update sprite col/row per frame
│   └── debug.rs            ── Debug collider visualization via effects pipeline
│
├── renderer/               ── BUFFER TYPES (Rust-side render data)
│   ├── instance.rs         ── RenderBuffer, RenderInstance (#[repr(C)], 8 floats, Pod)
│   ├── camera.rs           ── Camera2D (orthographic projection matrix builder)
│   └── sdf_instance.rs     ── SDFBuffer, SDFInstance (#[repr(C)], 12 floats, Pod)
│
├── bridge/                 ── CROSS-BOUNDARY PROTOCOL
│   └── protocol.rs         ── ProtocolLayout, header constants, wire format specs
│
├── extensions/             ── OPT-IN ADDITIONS (keyed by EntityId, no Entity mutation)
│   ├── easing.rs           ── 19 easing functions, pure math, Easing enum
│   ├── transform.rs        ── TransformGraph: parent-child hierarchy by EntityId
│   └── tween.rs            ── TweenState: animated value transitions by EntityId
│
├── input/                  ── INPUT ABSTRACTION
│   └── queue.rs            ── InputQueue, InputEvent enum (Pointer/Key/Custom)
│
└── assets/                 ── ASSET PIPELINE
    ├── manifest.rs         ── AssetManifest: JSON schema for atlases + named sprites
    └── registry.rs         ── SpriteRegistry: HashMap<String, SpriteComponent>
```

### 3.2 EngineContext Composition

```
EngineContext
├── scene: Scene                        ── Vec<Entity>, spawn/despawn/get/iter
├── effects: EffectsState               ── particle sim, arc generation, effects buffer
├── sounds: Vec<SoundEvent>             ── per-frame, cleared each tick
├── events: Vec<GameEvent>              ── per-frame, cleared each tick
├── camera: Camera2D                    ── orthographic projection, pan/zoom
├── lights: LightState                  ── persistent point lights, ambient color
├── bake: BakeState                     ── layer cache mask + generation counter
├── vectors: VectorState                ── [feature: vectors] lyon tessellation output
├── physics: PhysicsWorld               ── [feature: physics] rapier2d wrapper
├── collision_events: Vec<CollisionPair>── [feature: physics] per-step collision pairs
├── next_id: u32                        ── monotonic entity ID counter
└── sprite_registry: SpriteRegistry     ── name → SpriteComponent lookup
```

### 3.3 Fat Entity Layout

```
Entity (~200+ bytes with all options)
┌────────────────────────────────────────┐
│  id: EntityId(u32)              4B     │
│  tag: String                   24B+    │  ← heap-allocated, potential perf wart
│  active: bool                   1B     │
│  layer: RenderLayer             1B     │  ← enum: Background(0)..UI(5)
│  pos: Vec2                      8B     │
│  rotation: f32                  4B     │
│  scale: Vec2                    8B     │
│  sprite: Option<SpriteComponent>       │  ← 28B if Some
│  body: Option<PhysicsBody>             │  ← [physics] handle to rapier body
│  emitter: Option<EmitterComponent>     │  ← ~80B if Some (emission config)
│  mesh: Option<MeshComponent>           │  ← ~60B if Some (SDF shape + material)
│  animation: Option<AnimationComponent> │  ← ~32B if Some
└────────────────────────────────────────┘

Storage: Scene.entities: Vec<Entity>
 - Contiguous in memory (good L1 cache for iteration)
 - Lookup: O(n) linear scan by id or tag
 - Despawn: swap_remove (O(1), breaks order — fine, render sorts anyway)
```

---

## 4. SharedArrayBuffer Wire Protocol (v4.0)

### 4.1 Header Layout (28 floats = 112 bytes)

```
Offset  Field                      Written    Purpose
──────  ─────────────────────────  ─────────  ─────────────────────────────
  0     lock                       per-frame  Int32 Atomics sync (0=free, 1=ready)
  1     frame_counter              per-frame  monotonic frame number
  2     max_instances              once       capacity (from GameConfig)
  3     instance_count             per-frame  active count this frame
  4     atlas_split                per-frame  legacy: count of atlas-0 instances
  5     max_effects_vertices       once       capacity
  6     effects_vertex_count       per-frame  active count
  7     world_width                per-frame  game world dimensions
  8     world_height               per-frame
  9     max_sounds                 once       capacity
 10     sound_count                per-frame  active count
 11     max_events                 once       capacity
 12     event_count                per-frame  active count
 13     protocol_version           once       4.0
 14     max_sdf_instances          once       capacity
 15     sdf_instance_count         per-frame  active count
 16     max_vector_vertices        once       capacity
 17     vector_vertex_count        per-frame  active count
 18     max_layer_batches          once       capacity
 19     layer_batch_count          per-frame  active count
 20     layer_batch_data_offset    once       byte offset to batch section
 21     bake_state                 per-frame  encoded: mask | (generation << 6)
 22     max_lights                 once       capacity
 23     light_count                per-frame  active count
 24     ambient_r                  per-frame  scene ambient RGB
 25     ambient_g                  per-frame
 26     ambient_b                  per-frame
 27     wasm_time_us               per-frame  WASM tick duration (microseconds)
```

### 4.2 Body Sections (contiguous, offsets computed from capacities)

```
┌─────────────────────────────────────────────────────────────────────┐
│  HEADER [28 floats]                                                 │
├─────────────────────────────────────────────────────────────────────┤
│  INSTANCES [max_instances × 8 floats]                               │
│    per instance: x, y, rotation, scale, sprite_col, alpha,          │
│                  cell_span, atlas_row                               │
├─────────────────────────────────────────────────────────────────────┤
│  EFFECTS [max_effects_vertices × 5 floats]                          │
│    per vertex: x, y, z(color_idx), u, v                             │
├─────────────────────────────────────────────────────────────────────┤
│  SOUNDS [max_sounds × 1 float]                                      │
│    per event: event_id as f32                                       │
├─────────────────────────────────────────────────────────────────────┤
│  GAME EVENTS [max_events × 4 floats]                                │
│    per event: kind, a, b, c                                         │
├─────────────────────────────────────────────────────────────────────┤
│  SDF INSTANCES [max_sdf_instances × 12 floats]                      │
│    per instance: x, y, radius, rotation, r, g, b, shininess,        │
│                  emissive, shape_type, half_height, extra           │
├─────────────────────────────────────────────────────────────────────┤
│  VECTORS [max_vector_vertices × 6 floats]                           │
│    per vertex: x, y, r, g, b, a                                     │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER BATCHES [max_layer_batches × 4 floats]                       │
│    per batch: layer_id, start_idx, end_idx, atlas_id                │
├─────────────────────────────────────────────────────────────────────┤
│  LIGHTS [max_lights × 8 floats]                                     │
│    per light: x, y, r, g, b, intensity, radius, layer_mask          │
└─────────────────────────────────────────────────────────────────────┘

Default buffer size (GameConfig::default()):
  28 + (2048×8) + (16384×5) + 32 + (32×4) + (256×12) + (16384×6) + (48×4) + (64×8)
  = 28 + 16384 + 81920 + 32 + 128 + 3072 + 98304 + 192 + 512
  = 200,572 floats = 802,288 bytes ≈ 784 KB
```

### 4.3 Per-Frame Data Flow Through SAB

```
     WASM Linear Memory                    SharedArrayBuffer
     (wasmMemory.buffer)                   (sharedBuffer)

  ┌──────────────────────┐
  │  RenderBuffer        │
  │  ┌──────────────┐    │    Float32Array.set()     ┌──────────────────┐
  │  │ instances[]  │────┼──────────────────────────►│ SAB instances    │
  │  └──────────────┘    │   (memcpy, not zero-copy) │ section          │
  │                      │                           └──────────────────┘
  │  EffectsState        │
  │  ┌───────────────┐   │    Float32Array.set()     ┌──────────────────┐
  │  │ effects_buf[] │───┼──────────────────────────►│ SAB effects      │
  │  └───────────────┘   │                           │ section          │
  │                      │                           └──────────────────┘
  │  SDFBuffer           │
  │  ┌──────────────┐    │    Float32Array.set()     ┌──────────────────┐
  │  │ sdf_instances│────┼──────────────────────────►│ SAB SDF section  │
  │  └──────────────┘    │                           └──────────────────┘
  │                      │
  │  VectorState         │    Float32Array.set()     ┌──────────────────┐
  │  ┌──────────────┐    │                           │ SAB vectors      │
  │  │ vertex_buf[] │────┼──────────────────────────►│ section          │
  │  └──────────────┘    │                           └──────────────────┘
  │                      │
  │  LightState          │    Float32Array.set()     ┌──────────────────┐
  │  ┌──────────────┐    │  (repr(C) → raw f32*)     │ SAB lights       │
  │  │ lights[]     │────┼──────────────────────────►│ section          │
  │  └──────────────┘    │                           └──────────────────┘
  └──────────────────────┘

  After all copies: Atomics.store(sharedI32[0], 1) + Atomics.notify(sharedI32[0])
```

---

## 5. TypeScript Renderer Architecture (`packages/zap-web`)

### 5.1 Module Map

```
packages/zap-web/src/
│
├── index.ts               ── Public exports (initRenderer, ProtocolLayout, etc.)
│
├── worker/
│   ├── engine.worker.ts   ── Web Worker: loads WASM, runs game loop, writes SAB
│   ├── protocol.ts        ── ProtocolLayout class (TS mirror of Rust protocol.rs)
│   └── frame-reader.ts    ── readFrameState(): SAB → typed FrameState object
│
├── renderer/
│   ├── types.ts           ── Renderer interface, LayerBatchDescriptor, BakeState
│   ├── index.ts           ── initRenderer(): probes WebGPU → falls back to C2D
│   ├── camera.ts          ── buildProjectionMatrix(), computeProjection()
│   ├── constants.ts       ── shared render constants
│   │
│   ├── webgpu.ts          ── WebGPU renderer: orchestration facade
│   ├── webgpu/
│   │   ├── device.ts      ── initDevice(): adapter, device, surface config, HDR tier
│   │   ├── resources.ts   ── GPU buffers, textures, bind groups, atlas loading
│   │   ├── pipelines/
│   │   │   ├── common.ts  ── shared pipeline layout creation
│   │   │   ├── sprite.ts  ── sprite pipelines (one per atlas × blend mode)
│   │   │   ├── effects.ts ── additive glow pipeline
│   │   │   ├── sdf.ts     ── raymarched SDF pipeline + normal output
│   │   │   ├── vector.ts  ── CPU-tessellated polygon pipeline
│   │   │   └── lighting.ts── fullscreen post-process lighting pipeline
│   │   └── passes/
│   │       ├── bake.ts    ── render-to-texture for static layer caching
│   │       └── scene.ts   ── per-frame scene rendering + compositing
│   │
│   ├── canvas2d.ts        ── Canvas2D fallback renderer (full reimplementation)
│   ├── compositor.ts      ── LayerCompositor: multi-layer bake/compose logic
│   │
│   ├── shaders.wgsl       ── sprite + effects shaders
│   ├── molecule.wgsl      ── SDF raymarching shader (sphere/capsule/rounded-box)
│   ├── vector.wgsl        ── vector polygon shader
│   ├── lighting.wgsl      ── fullscreen point light accumulation + normal mapping
│   └── composite.wgsl     ── layer composition shader
│
├── react/
│   ├── index.ts           ── exports useZapEngine, GameEvent type
│   ├── useZapEngine.ts    ── React hook: lifecycle, input, resize, audio, rAF
│   └── TimingBars.tsx     ── performance visualization component
│
├── audio/
│   └── sound.ts           ── SoundManager: Web Audio API, event-driven playback
│
└── assets/
    ├── manifest.ts        ── AssetManifest JSON parser + loader
    └── normals.ts         ── Normal map blob loader
```

### 5.2 WebGPU Render Pipeline Ordering

```
Per frame (in rAF callback):

  1. readFrameState(sharedF32, layout) → FrameState
     │
     ▼
  2. Check bakeState — if generation changed, re-render baked layers
     │
     ▼
  3. Upload GPU buffers:
     │  instanceBuffer.writeBuffer(instanceData)
     │  effectsBuffer.writeBuffer(effectsData)
     │  sdfBuffer.writeBuffer(sdfData)
     │  vectorBuffer.writeBuffer(vectorData)
     │  lightBuffer.writeBuffer(lightData)
     │
     ▼
  4. SCENE PASS (render target: sceneTexture, rgba16float)
     │
     │  For each LayerBatch in layerBatches:
     │    ├─ skip if layer is baked (use cached texture)
     │    ├─ setPipeline(spritePipelines[batch.atlasId])
     │    ├─ setBindGroup(1, textureBindGroups[batch.atlasId])
     │    └─ drawIndexed(6, batch.end - batch.start, 0, 0, batch.start)
     │
     │  If sdfInstanceCount > 0:
     │    ├─ setPipeline(sdfPipeline)
     │    └─ drawIndexed(6, sdfInstanceCount)
     │
     │  If vectorVertexCount > 0:
     │    ├─ setPipeline(vectorPipeline)
     │    └─ draw(vectorVertexCount)
     │
     │  If effectsVertexCount > 0:
     │    ├─ setPipeline(effectsPipeline)  // additive blend
     │    └─ draw(effectsVertexCount)
     │
     ▼
  5. NORMAL PASS (render target: normalTexture, rgba8unorm)
     │  Same geometry as scene pass but uses fs_normal fragment shader
     │  Outputs tangent-space normals for deferred lighting
     │
     ▼
  6. LIGHTING PASS (render target: final surface texture)
     │  Fullscreen triangle
     │  Samples: sceneTexture + normalTexture
     │  Accumulates: ambient + Σ(pointLight × attenuation × NdotL)
     │  Output: lit scene color
     │
     ▼
  7. Present (GPUCanvasContext.getCurrentTexture())


HDR Tier Selection (at init, affects pipeline constants):
  ┌──────────────────────────────────────────────────────────────┐
  │ Probe on disposable canvas:                                  │
  │   Try rgba16float + display-p3 + extended → hdr-edr          │
  │   Try rgba16float + sRGB                  → hdr-srgb         │
  │   Fall back to preferredFormat            → sdr              │
  │   WebGPU unavailable                      → canvas2d         │
  │                                                              │
  │ Effect on shaders:                                           │
  │   hdr-edr:  EFFECTS_HDR_MULT=6.4, SDF_EMISSIVE_MULT=5.4      │
  │   hdr-srgb: EFFECTS_HDR_MULT=3.0, SDF_EMISSIVE_MULT=2.5      │
  │   sdr:      EFFECTS_HDR_MULT=1.0, SDF_EMISSIVE_MULT=0.5      │
  └──────────────────────────────────────────────────────────────┘
```

### 5.3 Layer Baking System

```
BakeState (Rust-side):
  mask: u8         ── bits 0-5 = which of 6 layers are baked
  generation: u32  ── incremented on bake/invalidate/unbake

Encoding for SAB (single f32):
  encoded = mask | (generation << 6)
  f32 can represent integers exactly up to 2^24 → ~262k generation changes

Main thread renderer:
  previousBakeGen = 0

  each frame:
    if bakeState.bakeGen != previousBakeGen:
      for each baked layer:
        render layer to intermediate texture (render-to-texture)
        cache texture
      previousBakeGen = bakeState.bakeGen

    during scene pass:
      for each layer:
        if layer is baked → draw cached texture (single fullscreen quad)
        else             → draw live entities normally
```

---

## 6. WASM Bridge (`crates/zap-web`)

### 6.1 `export_game!` Macro Expansion

```
Input:
  zap_web::export_game!(PhysicsPlayground, "physics-playground", vectors);

Expands to:

  thread_local! {
      static RUNNER: RefCell<Option<GameRunner<PhysicsPlayground>>> = RefCell::new(None);
  }

  fn with_runner<R>(f: impl FnOnce(&mut GameRunner<PhysicsPlayground>) -> R) -> R { ... }

  #[wasm_bindgen] pub fn game_init()                          { ... }
  #[wasm_bindgen] pub fn game_tick(dt: f32)                   { ... }
  #[wasm_bindgen] pub fn game_pointer_down(x: f32, y: f32)    { ... }
  #[wasm_bindgen] pub fn game_pointer_up(x: f32, y: f32)      { ... }
  #[wasm_bindgen] pub fn game_pointer_move(x: f32, y: f32)    { ... }
  #[wasm_bindgen] pub fn game_key_down(key_code: u32)         { ... }
  #[wasm_bindgen] pub fn game_key_up(key_code: u32)           { ... }
  #[wasm_bindgen] pub fn game_custom_event(kind, a, b, c)     { ... }
  #[wasm_bindgen] pub fn game_load_manifest(json: &str)       { ... }
  #[wasm_bindgen] pub fn get_instances_ptr() -> *const f32    { ... }  // 18 more accessors
  // + vector accessors (from "vectors" variant)

  Total: ~40 #[wasm_bindgen] exports per game
```

### 6.2 GameRunner Tick Sequence

```
GameRunner::tick(dt):
  │
  ├─ 1. ctx.clear_frame_data()              ── reset sounds, events, collisions, vectors
  │
  ├─ 2. timestep.accumulate(dt) → N steps   ── fixed timestep accumulator
  │     │
  │     └─ for each step:
  │         ├─ game.update(&mut ctx, &input) ── user game logic (forces, spawns, queries)
  │         │
  │         ├─ for each physics substep:     ── [physics] e.g. 4× = 240Hz
  │         │    ctx.step_physics()           ── rapier step + pos sync + collision collect
  │         │
  │         ├─ tick_emitters(&scene, &effects)── entity emitters → particle spawns
  │         └─ effects.tick(dt)              ── particle physics, arc aging, buffer rebuild
  │
  ├─ 3. input.drain()                       ── clear consumed input events
  │
  ├─ 4. build_render_buffer(scene, &mut rb)  ── sort entities by (layer, atlas), pack f32s
  │     → returns Vec<LayerBatch>
  │
  ├─ 5. serialize layer_batches → f32 buffer
  │
  ├─ 6. build_sdf_buffer(scene, &mut sdf)   ── Entity.mesh → SDFInstance packing
  │
  ├─ 7. game.render(&mut render_ctx)         ── optional: custom render commands
  │
  ├─ 8. effects.rebuild_effects_buffer()     ── particles + arcs → triangle vertices
  │
  └─ 9. pack sound events → flat buffer
```

---

## 7. Game Loop Timing

```
                     Worker                          Main Thread
                       │                                │
                       │  setTimeout(gameLoop, 16)      │  requestAnimationFrame
                       │  (~60fps, NOT vsync-locked)    │  (vsync-locked)
                       │                                │
 Frame N:              │                                │
   game_tick(1/60) ────┤                                │
   get_instance_count()│                                │
   get_instances_ptr() │                                │
   ... (25 FFI calls)  │                                │
   F32Array.set(SAB)   │                                │
   Atomics.store(1) ───┼──────────────────────►         │
   Atomics.notify()    │                          readFrameState(SAB)
                       │                          upload GPU buffers
   setTimeout(16ms) ───┤                          encode render passes
                       │                          present
                       │                                │
 Frame N+1:            │                                │
   game_tick(1/60) ────┤                          rAF callback ──────┤
   ...                 │                                │

 Timing contract:
   Worker: ~16ms per tick (setTimeout, can drift ±2ms)
   Main thread: vsync-locked rAF (~16.67ms @ 60Hz)
   No explicit synchronization beyond Atomics.store/notify
   Risk: if WASM tick > 16ms, frames pile up or render stale data
```

---

## 8. Physics Integration Detail

```
PhysicsWorld (wraps rapier2d 0.22)
├── pipeline: PhysicsPipeline
├── rigid_body_set: RigidBodySet
├── collider_set: ColliderSet
├── broad_phase: DefaultBroadPhase
├── narrow_phase: NarrowPhase
├── island_manager: IslandManager
├── impulse_joint_set: ImpulseJointSet
├── ccd_solver: CCDSolver
├── query_pipeline: QueryPipeline
├── dt: f32 (physics_dt = fixed_dt / substeps)
├── gravity: Vec2
└── body_map: Vec<(EntityId, RigidBodyHandle)>   ── entity→rapier lookup

DirectEventCollector (WASM-safe):
  Uses RefCell<Vec<CollisionEvent>> instead of crossbeam ChannelEventCollector.
  crossbeam depends on pthreads → does not compile to wasm32-unknown-unknown.
  RefCell is single-threaded (WASM is single-threaded within a worker) → safe.

step_into(&mut collision_pairs):
  pipeline.step(gravity, dt, &bodies, &colliders, ..., &event_collector)
  │
  ├── sync positions: for each (entity_id, body_handle) in body_map:
  │     entity.pos = body.translation()
  │     entity.rotation = body.rotation()
  │
  └── collect collisions: for each event in event_collector:
        resolve body.user_data (u128 → EntityId) for both bodies
        push CollisionPair { entity_a, entity_b, started: bool }

Collision event delay:
  step N events → visible to game in step N+1
  At 60fps = 16.6ms delay (acceptable for casual games)
```

---

## 9. Example Game Architectures

### 9.1 Common Structure (all examples)

```
examples/<game>/
├── Cargo.toml             ── deps: zap-engine, zap-web, wasm-bindgen
├── src/
│   ├── lib.rs             ── zap_web::export_game!(GameStruct, "name")
│   └── game.rs            ── impl Game for GameStruct { config, init, update }
├── App.tsx                ── React UI overlay
├── main.tsx               ── ReactDOM.render(<App />)
├── index.html             ── entry point, loads main.tsx
└── public/assets/         ── sprites, atlases, sounds

Build: wasm-pack build examples/<game> --target web --out-dir pkg
Run:   npm run dev → Vite serves at :5173 with COOP/COEP headers
```

### 9.2 Basic Demo (simplest example)

```
BasicDemo: bouncing physics sprites in a box

  config():
    800×600 world, gravity (0, 981), 60fps

  init(ctx):
    spawn 4 invisible fixed-body walls (boundary box)
    spawn 8 dynamic-body sprites in grid (ball colliders, restitution=0.8)

  update(ctx, input):
    on PointerDown → spawn new physics sprite at click pos + particle burst + sound
    for each new collision → spawn spark particles at midpoint

  Features used: physics (rapier2d), sprites, effects (particles), sounds
  Features NOT used: SDF, vectors, lighting, baking, extensions

  Data flow:
    click → postMessage → worker → game_pointer_down → InputEvent::PointerDown
    → game.update reads input → ctx.spawn_with_body() + ctx.effects.spawn_particles()
    → tick finishes → build_render_buffer → SAB → main thread renders
```

### 9.3 Physics Playground (Angry Birds clone)

```
PhysicsPlayground: slingshot + tower destruction

  State machine: Aiming → Flying → Settled → Aiming

  config():
    1200×600 world, gravity (0, 600), 60fps, vectors enabled

  init(ctx):
    ground (fixed body, half_height=25)
    walls (fixed bodies, left+right)
    tower: 5×3 grid of dynamic cuboid blocks at x=800
    projectile: dynamic ball at sling anchor (400, 450)

  update(ctx, input):
    Aiming:
      drag → track drag vector
      release → apply impulse (drag_delta × LAUNCH_SCALE) → transition to Flying
      draw sling rubber bands via ctx.vectors (lyon polygons)

    Flying:
      track projectile velocity
      if velocity < threshold for 60 frames → Settled
      if flight_timer > 600 frames → auto-reset (prevents stuck projectiles)
      on collision with blocks → emit score events to React, spawn sparks

    Settled:
      respawn projectile at sling anchor → Aiming

  React App.tsx:
    receives GameEvent(kind=1, a=score) via onGameEvent callback
    displays score, reset button (sends CustomEvent kind=1 back to WASM)

  Bidirectional event flow:
    React → worker: postMessage({type:'custom', kind:1}) → game_custom_event(1,0,0,0)
    WASM → React:   ctx.emit_event(GameEvent{kind:1.0, a:score}) → postMessage({type:'event'})
```

### 9.4 Chemistry Lab (SDF raymarching)

```
ChemistryLab: interactive molecule builder

  config():
    1200×800 world, no gravity, physics enabled (spring joints for bonds)

  Architecture (game-side modules):
    game.rs        ── Game trait impl, event dispatch
    chemistry.rs   ── element data, bond logic
    molecule.rs    ── molecule state, atom graph
    molecule3d.rs  ── 3D→2D projection for orbital visualization
    physics.rs     ── spring joint management between atoms
    render3d.rs    ── pseudo-3D rotation math
    renderer.rs    ── entity creation for atoms/bonds
    sim.rs         ── simulation step (rotation, bonds, forces)
    vsepr.rs       ── VSEPR geometry (lone pairs, bond angles)
    bohr.rs        ── Bohr model electron shell visualization
    periodic_table.rs ── in-game periodic table

  Rendering:
    Atoms: Entity.mesh = MeshComponent { shape: SDFShape::Sphere { radius }, ... }
    Bonds: Entity.mesh = MeshComponent { shape: SDFShape::Capsule { half_height }, ... }
    → build_sdf_buffer → SDFInstance buffer → molecule.wgsl raymarching
    Per-fragment: SDF evaluation → normal → Phong + Fresnel + HDR emissive

  React App.tsx:
    PeriodicTable.tsx (interactive element selector)
    CameraControls.tsx (pan/zoom via custom events)
    Molecule info display

  Features used: SDF (sphere+capsule), physics (spring joints), custom events
  Features NOT used: sprites (no texture atlases), effects, lighting
```

### 9.5 ZapZap Mini (circuit puzzle + dynamic lighting)

```
ZapZapMini: 8×8 circuit tile puzzle

  config():
    800×800 world, no gravity, vectors enabled

  Architecture (game-side modules):
    game.rs       ── Game trait impl, tap handling, win detection
    board.rs      ── 8×8 tile grid, connection logic, pathfinding
    animation.rs  ── tile rotation animation (tweened)

  Rendering pipeline:
    Tile board: sprites from atlas (base_tiles_16x8.png, arrows_8x8.png)
    Electric arcs: when path connects left→right edges
      → ctx.effects.spawn_electric_arc(from, to, segments, color)
      → midpoint displacement algorithm
      → triangle strip → additive HDR glow shader
    Dynamic lights: PointLight at each arc endpoint
      → ctx.lights.add(PointLight::new(...))
      → lighting.wgsl accumulates contributions
    Normal maps: tiles have pre-baked normal maps (Sobel operator)
      → fs_normal shader outputs to G-buffer
      → lighting pass reads normals for N·L directional shading
    Layer baking: Background layer baked (static tiles)
      → ctx.bake_layer(RenderLayer::Background)
      → invalidate on tile rotation

  React App.tsx:
    Timer, move counter, level selector
    Receives win event → shows completion UI

  Features used: sprites, effects (arcs), lighting (point lights + normals),
                 layer baking, custom events, vectors (debug)
  Full rendering pipeline activated (most complex example)
```

---

## 10. Asset Pipeline

```
  Developer workflow:
    1. Drop images into examples/<game>/public/assets/
    2. Naming convention: hero_4x8.png → 4-col × 8-row atlas
                          bullet.png   → 1×1 single sprite

    3. npm run bake-assets (runs tools/bake-assets.ts)
       ├── scans directory
       ├── detects atlas dimensions from filename pattern *_NxM.ext
       └── outputs assets.json:
           {
             "atlases": [
               { "name": "hero", "cols": 4, "rows": 8, "path": "hero_4x8.png" }
             ],
             "sprites": {
               "hero_0_0": { "atlas": 0, "col": 0, "row": 0 },
               "hero_1_0": { "atlas": 0, "col": 1, "row": 0 },
               ...
             }
           }

    4. Runtime loading:
       React hook → fetch(assetsUrl) → loadManifest() → AssetManifest
                                      → loadAssetBlobs() → Image objects
                                      → loadNormalMapBlobs() → normal Images

       Worker init → manifestJson via postMessage
                   → game_load_manifest(json) → wasm-bindgen
                   → EngineContext::load_manifest() → SpriteRegistry::from_manifest()
                   → ctx.sprite("hero_0_0") returns SpriteComponent { atlas:0, col:0, row:0 }

       Renderer init → atlas Images → GPU textures (one per atlas)
                     → createSpritePipelines() (one pipeline per atlas, with ATLAS_COLS/ROWS overrides)
                     → createTextureBindGroups() (one bind group per atlas)
```

---

## 11. Input Path (end-to-end latency)

```
  DOM Event (user clicks canvas)
    │
    ▼  ~0ms
  useZapEngine onPointerDown handler
    │  compute CSS coordinates
    │
    ▼  ~0.1ms (postMessage serialization)
  worker.postMessage({ type: 'pointer_down', x, y })
    │
    ▼  ~0.1ms (worker message dispatch)
  engine.worker.ts onmessage handler
    │  screenToWorld(cssX, cssY) → world coords
    │
    ▼  ~0.01ms (wasm-bindgen FFI call)
  wasm.game_pointer_down(worldX, worldY)
    │
    ▼  ~0ms (in-memory push)
  GameRunner.push_input(InputEvent::PointerDown { x, y })
    │  stored in InputQueue
    │
    ▼  next game_tick call (up to 16ms later)
  game.update(ctx, input)
    │  game reads input.iter() and acts on it
    │
    ▼  same tick
  build_render_buffer → SAB write → Atomics.notify
    │
    ▼  next rAF (up to 16ms later)
  renderer reads SAB → draws frame

  Total worst-case latency: ~32ms (2 frames at 60fps)
  Typical latency: ~16-24ms (1-1.5 frames)
```

---

## 12. Extensions Module (opt-in, decoupled)

```
Extensions operate on EntityId keys, never hold Entity references.
Game manages extension state alongside EngineContext.

  ┌──────────────────────────────────────────────────────┐
  │  Game struct                                         │
  │  ├── tweens: TweenState      ── animated transitions │
  │  ├── transforms: TransformGraph ── parent-child      │
  │  └── (EngineContext via ctx)                         │
  │                                                      │
  │  update(ctx, input):                                 │
  │    tweens.tick(dt, &mut ctx.scene)                   │
  │    transforms.propagate(&mut ctx.scene)              │
  └──────────────────────────────────────────────────────┘

  TweenState:
    HashMap<EntityId, Vec<Tween>>
    Tween { prop: TweenProp, from, to, duration, elapsed, easing }
    tick(dt, scene): for each tween, interpolate → write to entity via scene.get_mut(id)

  TransformGraph:
    HashMap<EntityId, LocalTransform>  ── offset, rotation, scale relative to parent
    HashMap<EntityId, Option<EntityId>> ── parent links
    propagate(scene): walk tree, compose transforms, write to entity.pos/rotation

  Easing (pure math, no state):
    19 functions: Linear, QuadIn/Out/InOut, CubicIn/Out/InOut, QuartIn/Out/InOut,
                  BackIn/Out/InOut, ElasticOut, BounceOut, SineIn/Out/InOut
    apply(t: f32) → f32
```
