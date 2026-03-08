# ZapEngine: Complete Lifecycle & Data Flow Reference

## 1. The Texture/Sprite Question: What WASM Knows vs What It Doesn't

**WASM has zero access to texture pixels, image data, or pixel dimensions.**

The WASM module operates on a completely abstract sprite model. Here is exactly what each side knows:

```
WASM (Rust) knows:                         Main Thread (TypeScript) knows:
─────────────────                          ──────────────────────────────
SpriteComponent:                           GPUTextureAsset:
  atlas: AtlasId(u32)    ← index           texture: GPUTexture     ← actual pixels
  col: f32               ← grid cell       view: GPUTextureView
  row: f32               ← grid cell       width: number           ← pixel width
  cell_span: f32         ← how many cells  height: number          ← pixel height
  alpha: f32
  blend: BlendMode                         AssetManifest (shared):
                                             atlases[i].cols: 16
Entity.scale: Vec2       ← world units      atlases[i].rows: 8
Entity.pos: Vec2         ← world position    atlases[i].path: "tiles.png"
Entity.rotation: f32     ← radians

RenderInstance (written to SAB):           Shader (shaders.wgsl):
  x, y, rotation, scale                     ATLAS_COLS override ← from manifest
  sprite_col, alpha                          ATLAS_ROWS override ← from manifest
  cell_span, atlas_row                       UV = (col/COLS, row/ROWS) + uv * (span/COLS, span/ROWS)
```

The manifest JSON is the only shared schema. It contains grid dimensions (cols, rows) and file paths — but not pixel sizes. WASM receives the manifest JSON to build a `SpriteRegistry` (name → grid coordinates). The main thread receives the same manifest to load actual PNG files into GPU textures and configure shader override constants.

**The WASM module never sees:**
- Raw image bytes or pixels
- Texture dimensions in pixels
- GPU texture handles or views
- The actual PNG files
- Which rendering backend is being used (WebGPU or Canvas2D)

**Why this works:** The WASM side writes `sprite_col=3.0, atlas_row=2.0` into the SAB. The shader reads those values and computes UV coordinates using the `ATLAS_COLS`/`ATLAS_ROWS` overrides that were set at pipeline creation from the manifest. The WASM module doesn't need to know the texture is 1024×512px — it only needs to know the sprite is at grid cell (3, 2) in an atlas with 16 columns and 8 rows.

**Where this creates a limitation:** WASM cannot do pixel-accurate collision, sprite bounding-box queries, or any operation that depends on the actual rendered size of a sprite in pixels. It knows the world-space `scale` (e.g., "50 world units") but not how many pixels that maps to on screen. The screen mapping happens entirely in the camera projection matrix on the main thread.

---

## 2. Complete Input → GPU Lifecycle

Tracing a pointer click from DOM event to GPU draw call, with every intermediate step.

### Phase 1: DOM → React Hook

```
User clicks canvas
    │
    ▼
DOM PointerEvent fires on <canvas>
    │  e.offsetX = 340, e.offsetY = 220  (CSS pixels, relative to canvas)
    │
    ▼
useZapEngine.onPointerDown(e)                    [useZapEngine.ts:246]
    │  1. soundManagerRef.current?.resume()       ← unsuspend AudioContext on first click
    │  2. workerRef.current?.postMessage({
    │       type: 'pointer_down',
    │       x: e.offsetX,                        ← CSS pixels, NOT world coordinates
    │       y: e.offsetY
    │     })
    │
    ▼
postMessage serialization (~0.1ms)
    │  structured clone of { type, x, y }
    │  crosses thread boundary via browser message queue
```

### Phase 2: Worker → WASM FFI

```
worker self.onmessage fires                      [engine.worker.ts:404]
    │
    ▼
case 'pointer_down':
    │  1. screenToWorld(e.data.x, e.data.y)      [engine.worker.ts:117]
    │     │
    │     │  computeProjection(canvasWidth, canvasHeight, worldWidth, worldHeight)
    │     │    projWidth = aspect-corrected visible world width
    │     │    projHeight = aspect-corrected visible world height
    │     │
    │     │  worldX = cssX * projWidth / canvasWidth
    │     │  worldY = cssY * projHeight / canvasHeight
    │     │
    │     │  Example: canvas 800×600 CSS, world 1200×600
    │     │    projWidth = 1200, projHeight = 600 (aspect matches)
    │     │    worldX = 340 * 1200 / 800 = 510.0
    │     │    worldY = 220 * 600 / 600 = 220.0
    │     │
    │     ▼  returns { x: 510.0, y: 220.0 }
    │
    │  2. wasm.game_pointer_down(510.0, 220.0)    ← wasm-bindgen FFI call (~20ns)
    │
    ▼
#[wasm_bindgen] game_pointer_down(x, y)          [lib.rs via export_game! macro]
    │
    │  with_runner(|r| r.push_input(InputEvent::PointerDown { x: 510.0, y: 220.0 }))
    │
    ▼
InputQueue.push(event)                           ← stored in Vec<InputEvent>
    │  event sits in queue until next game_tick
```

### Phase 3: Game Tick Consumes Input

```
Worker game loop calls wasm.game_tick(1/60)      [engine.worker.ts:270]
    │
    ▼
GameRunner::tick(dt)                             [runner.rs:101]
    │
    ├─ ctx.clear_frame_data()                    ← clear sounds, events, collisions
    │
    ├─ timestep.accumulate(dt) → 1 step
    │     │
    │     ├─ game.update(&mut ctx, &input)       [game.rs:288]
    │     │     │
    │     │     │  for event in input.iter():
    │     │     │    match InputEvent::PointerDown { x: 510.0, y: 220.0 }:
    │     │     │      // Game-specific logic. Example (PhysicsPlayground):
    │     │     │      //   check distance to sling anchor
    │     │     │      //   if < 100 units → start dragging
    │     │     │      //   spawn particle burst
    │     │     │      //   ctx.emit_sound(SoundEvent(0))
    │     │     │      //   ctx.emit_event(GameEvent { kind: 1.0, a: score, ... })
    │     │     │
    │     │     ▼  game state mutated: entities spawned/moved, forces applied
    │     │
    │     ├─ ctx.step_physics()                  ← rapier2d step, position sync
    │     │     each rapier body → entity.pos = body.translation()
    │     │
    │     ├─ tick_emitters(&scene, &effects)     ← entity emitters → particle spawns
    │     │
    │     └─ effects.tick(dt)                    ← particle physics, arc aging
    │
    ├─ input.drain()                             ← consumed, clear queue
    │
    ├─ build_render_buffer(scene, &mut rb)        [render.rs:29]
    │     │
    │     │  for each entity (active, has sprite):
    │     │    pack RenderInstance { x, y, rotation, scale, sprite_col, alpha, cell_span, atlas_row }
    │     │
    │     │  sort entries by (layer, atlas_id)
    │     │
    │     │  emit LayerBatch descriptors:
    │     │    { layer: Background, start: 0, end: 12, atlas_id: 0 }
    │     │    { layer: Objects, start: 12, end: 45, atlas_id: 0 }
    │     │    { layer: Objects, start: 45, end: 52, atlas_id: 1 }
    │     │    ...
    │     │
    │     ▼  RenderBuffer.instances: Vec<RenderInstance> (contiguous f32s in WASM heap)
    │
    ├─ build_sdf_buffer(scene, &mut sdf)         ← mesh entities → SDFInstance buffer
    │
    ├─ game.render(&mut render_ctx)              ← optional custom render commands
    │
    ├─ effects.rebuild_effects_buffer()          ← particles + arcs → triangle vertices
    │
    └─ pack sounds → sound_buffer
```

### Phase 4: WASM Heap → SharedArrayBuffer

```
Back in engine.worker.ts gameLoop()              [engine.worker.ts:264]
    │
    │  // Read counts from WASM
    │  const instanceCount = wasm.get_instance_count()       ← FFI call → returns 45
    │  const effectsVertexCount = wasm.get_effects_vertex_count()
    │  const sdfCount = wasm.get_sdf_instance_count()
    │  const soundLen = wasm.get_sound_events_len()
    │  const eventLen = wasm.get_game_events_len()
    │  const lightCount = wasm.get_light_count()
    │  ... (~25 FFI calls total, each ~20ns)
    │
    │  // Write header fields to SAB
    │  sharedF32[HEADER_INSTANCE_COUNT] = 45
    │  sharedF32[HEADER_EFFECTS_VERTEX_COUNT] = effectsVertexCount
    │  sharedF32[HEADER_WORLD_WIDTH] = 1200.0
    │  ... (13 header fields per frame)
    │
    │  // COPY instance data: WASM heap → SAB
    │  const ptr = wasm.get_instances_ptr()                  ← raw pointer into Rust Vec
    │  const wasmData = new Float32Array(
    │      wasmMemory.buffer,                                ← WASM linear memory ArrayBuffer
    │      ptr,                                              ← byte offset
    │      instanceCount * 8                                 ← 45 instances × 8 floats = 360 floats
    │  )
    │  sharedF32.set(wasmData, layout.instanceDataOffset)    ← memcpy: 360 × 4 = 1440 bytes
    │
    │  // COPY effects, SDF, vectors, layer batches, lights (same pattern)
    │  // Each is: get pointer → create Float32Array view → .set() into SAB
    │
    │  // Sound events go via postMessage (not SAB — too small to justify shared memory)
    │  if (soundLen > 0):
    │    const soundData = new Uint8Array(wasmMemory.buffer, soundPtr, soundLen)
    │    self.postMessage({ type: 'sound', events: Array.from(soundData) })
    │
    │  // Game events also via postMessage
    │  if (eventLen > 0):
    │    self.postMessage({ type: 'event', events: [...parsed events...] })
    │
    │  // Signal frame ready
    │  Atomics.store(sharedI32, 0, 1)                        ← write lock=1
    │  Atomics.notify(sharedI32, 0)                          ← wake main thread if waiting
    │
    ▼
setTimeout(gameLoop, 16)                                     ← schedule next tick
```

### Phase 5: Main Thread Reads SAB → GPU Draw

```
requestAnimationFrame callback fires             [useZapEngine.ts:364]
    │
    ├─ const buf = sharedF32Ref.current           ← Float32Array view of SAB
    │
    ├─ readFrameState(buf, layout)                [frame-reader.ts:65]
    │     │
    │     │  Read header:
    │     │    instanceCount = buf[3]              = 45
    │     │    effectsVertexCount = buf[6]
    │     │    sdfInstanceCount = buf[15]
    │     │    layerBatchCount = buf[19]
    │     │    lightCount = buf[23]
    │     │    wasmTimeUs = buf[27]
    │     │
    │     │  Create subarray VIEWS (zero-copy — just pointer + offset + length):
    │     │    instanceData = buf.subarray(28, 28 + 45*8)    ← no copy, just a view
    │     │    effectsData = buf.subarray(...)
    │     │    sdfData = buf.subarray(...)
    │     │    vectorData = buf.subarray(...)
    │     │
    │     │  Parse layer batches (small loop, creates JS objects):
    │     │    for i in 0..layerBatchCount:
    │     │      { layerId, start, end, atlasId }
    │     │
    │     │  Decode bake state: raw & 0x3F = mask, raw >>> 6 = generation
    │     │
    │     │  Extract lighting: lightData subarray + ambient RGB
    │     │
    │     ▼  returns FrameState object
    │
    ├─ renderer.draw(                             [webgpu.ts]
    │     instanceData, instanceCount, atlasSplit,
    │     effectsData, effectsVertexCount,
    │     sdfData, sdfInstanceCount,
    │     vectorData, vectorVertexCount,
    │     layerBatches, bakeState, lightingState
    │  )
    │
    │  INSIDE renderer.draw():
    │
    │  1. UPLOAD BUFFERS (CPU → GPU):
    │     device.queue.writeBuffer(instanceBuffer, 0, instanceData)
    │       ← 45 × 32 bytes = 1440 bytes uploaded to GPU storage buffer
    │     device.queue.writeBuffer(effectsBuffer, 0, effectsData)
    │     device.queue.writeBuffer(sdfStorageBuffer, 0, sdfData)
    │     device.queue.writeBuffer(vectorBuffer, 0, vectorData)
    │     device.queue.writeBuffer(lightStorageBuffer, 0, lightData)
    │     device.queue.writeBuffer(cameraBuffer, 0, projectionMatrix)
    │
    │  2. CHECK BAKE STATE:
    │     if bakeState.bakeGen !== previousBakeGen:
    │       for each baked layer:
    │         render-to-texture (cache as intermediate texture)
    │       previousBakeGen = bakeState.bakeGen
    │
    │  3. ENCODE SCENE RENDER PASS:
    │     const encoder = device.createCommandEncoder()
    │     const pass = encoder.beginRenderPass({
    │       colorAttachments: [{ view: sceneTextureView, loadOp: 'clear' }]
    │     })
    │
    │     for each layerBatch in layerBatches:
    │       if layer is baked → draw cached texture quad
    │       else:
    │         pass.setPipeline(spritePipelines[batch.atlasId])
    │           ← each pipeline has ATLAS_COLS/ATLAS_ROWS baked into shader
    │         pass.setBindGroup(0, cameraBindGroup)
    │         pass.setBindGroup(1, textureBindGroups[batch.atlasId])
    │           ← this is where the ACTUAL TEXTURE is bound
    │         pass.setBindGroup(2, instanceBindGroup)
    │           ← this is the storage buffer with instance data
    │         pass.drawIndexed(6, batch.end - batch.start, 0, 0, batch.start)
    │           ← 6 indices per quad (two triangles)
    │           ← batch.start = first instance index
    │           ← (batch.end - batch.start) = instance count for this batch
    │
    │     // SDF pass (if any)
    │     pass.setPipeline(sdfPipeline)
    │     pass.drawIndexed(6, sdfInstanceCount)
    │
    │     // Vector pass (if any)
    │     pass.setPipeline(vectorPipeline)
    │     pass.draw(vectorVertexCount)
    │
    │     // Effects pass (additive blend)
    │     pass.setPipeline(effectsPipeline)
    │     pass.draw(effectsVertexCount)
    │
    │     pass.end()
    │
    │  4. ENCODE NORMAL PASS (if normal maps exist):
    │     Renders same geometry to normalTexture using fs_normal fragment shader
    │
    │  5. ENCODE LIGHTING PASS (if lights > 0):
    │     Fullscreen triangle
    │     Samples sceneTexture + normalTexture
    │     Accumulates: ambient + Σ(light.color * intensity * attenuation * NdotL)
    │
    │  6. SUBMIT:
    │     device.queue.submit([encoder.finish()])
    │
    │  7. PRESENT:
    │     Browser composites canvas to screen on next vsync
    │
    ▼
requestAnimationFrame(frame)                     ← schedule next render
```

### Phase 5b: The Shader Resolves Grid → UV → Pixel

Inside the GPU, per vertex (6 per sprite, but only 4 unique via index buffer):

```
vs_main (vertex shader)                          [shaders.wgsl:82]
    │
    │  inst = instances[instance_index]           ← read from storage buffer
    │    inst.sprite_col = 3.0
    │    inst.atlas_row  = 2.0
    │    inst.scale      = 40.0
    │    inst.position   = (510.0, 220.0)
    │    inst.rotation   = 0.0
    │
    │  // Compute quad vertex position
    │  QUAD_POS[vertex] × scale + position → world_pos
    │  camera.projection × world_pos → clip_position
    │
    │  // Compute atlas UV from grid coordinates
    │  col = sprite_col % ATLAS_COLS              = 3.0 % 16.0 = 3.0
    │  row = atlas_row                            = 2.0
    │  cell_size = max(cell_span, 1.0)            = 1.0
    │
    │  uv_origin = (col / ATLAS_COLS, row / ATLAS_ROWS)
    │            = (3.0 / 16.0, 2.0 / 8.0)
    │            = (0.1875, 0.25)
    │
    │  uv_size = (cell_size / ATLAS_COLS, cell_size / ATLAS_ROWS)
    │          = (1.0 / 16.0, 1.0 / 8.0)
    │          = (0.0625, 0.125)
    │
    │  tex_coord = uv_origin + QUAD_UV[vertex] * uv_size
    │    corner (0,0): (0.1875, 0.25)                  ← top-left of cell
    │    corner (1,1): (0.1875 + 0.0625, 0.25 + 0.125) ← bottom-right of cell
    │
    ▼
fs_main (fragment shader)                        [shaders.wgsl:124]
    │
    │  textureSample(t_atlas, s_atlas, tex_coord)
    │    ← samples the ACTUAL GPU TEXTURE at the computed UV
    │    ← bilinear filtering via sampler
    │    ← returns RGBA color
    │
    │  return color * alpha                       ← final pixel color
```

---

## 3. WASM → React Event Lifecycle

Tracing a game event from Rust `emit_event` to React state update.

```
RUST (inside game.update):
    ctx.emit_event(GameEvent { kind: 1.0, a: 12.0, b: 0.0, c: 0.0 })
      │
      ▼
    ctx.events.push(GameEvent { kind: 1.0, a: 12.0, b: 0.0, c: 0.0 })
      │  stored in Vec<GameEvent> (Pod, #[repr(C)], 4 floats per event)
      │
      ▼
GameRunner::tick() finishes
      │  events remain in ctx.events until worker reads them
      │
      ▼
Worker gameLoop():                                [engine.worker.ts:356]
    const eventLen = wasm.get_game_events_len()   ← FFI: returns 1
    const ptr = wasm.get_game_events_ptr()        ← FFI: raw pointer into Vec<GameEvent>

    // Read 4 floats per event from WASM linear memory
    const eventData = new Float32Array(wasmMemory.buffer, ptr, 1 * 4)
      eventData = [1.0, 12.0, 0.0, 0.0]

    // Parse into JS objects
    const events = [{
      kind: 1.0,                                  ← eventData[0]
      a: 12.0,                                    ← eventData[1]
      b: 0.0,                                     ← eventData[2]
      c: 0.0                                      ← eventData[3]
    }]

    // Send via postMessage (NOT via SAB — events need to reach React)
    self.postMessage({ type: 'event', events })
      │
      ▼
postMessage crosses thread boundary
      │
      ▼
useZapEngine worker.onmessage handler            [useZapEngine.ts:233]
    case 'event':
      onGameEventRef.current?.(e.data.events)
        │
        ▼
React component's onGameEvent callback           [App.tsx:15]
    for (const e of events):
      if (e.kind === 1):
        setScore(e.a)                             ← React state update → re-render
          │
          ▼
React re-renders HUD:
    <div>Score: 12 / 15</div>
```

**Critical detail:** Game events go through `postMessage`, not through the SAB. The SAB does have an events section, but the worker reads the events from WASM memory and sends them via `postMessage` for two reasons: (1) React needs to receive them as structured objects for `onGameEvent` callback, and (2) the SAB events section would be overwritten next frame before React could process them. The SAB events section exists but is currently redundant — the `postMessage` path is the one that actually delivers events to the application.

---

## 4. React → WASM Custom Event Lifecycle

Tracing a button click in React that sends a command to the WASM game.

```
User clicks "Reset" button in React UI
    │
    ▼
React onClick handler                            [App.tsx:31]
    sendEvent({ type: 'custom', kind: 1 })
      │
      ▼
useZapEngine.sendEvent()                         [useZapEngine.ts:130]
    workerRef.current?.postMessage({ type: 'custom', kind: 1 })
      │
      ▼
postMessage crosses thread boundary
      │
      ▼
worker self.onmessage                            [engine.worker.ts:440]
    case 'custom':
      wasm.game_custom_event(
        e.data.kind ?? 0,                         = 1
        e.data.a ?? 0,                            = 0
        e.data.b ?? 0,                            = 0
        e.data.c ?? 0                             = 0
      )
        │
        ▼
#[wasm_bindgen] game_custom_event(1, 0, 0, 0)   [export_game! macro]
    with_runner(|r| r.push_input(InputEvent::Custom { kind: 1, a: 0.0, b: 0.0, c: 0.0 }))
      │
      ▼
InputQueue.push(InputEvent::Custom { kind: 1, ... })
      │  sits in queue until next tick
      │
      ▼
Next game_tick() → game.update(ctx, input):      [game.rs:292]
    for event in input.iter():
      match InputEvent::Custom { kind: 1, .. }:
        self.reset_level(ctx)                     ← game responds to the command
```

**The custom event protocol is 4 floats: (kind, a, b, c).** All game-specific. There is no schema — each game defines its own kind values. The engine treats them as opaque pass-through.

---

## 5. Sound Event Lifecycle

```
RUST (inside game.update):
    ctx.emit_sound(SoundEvent(0))                 ← event ID 0
      │
      ▼
    ctx.sounds.push(SoundEvent(0))                ← Vec<SoundEvent>, cleared each frame
      │
      ▼
GameRunner::tick() → pack sounds:                [runner.rs:154]
    self.sound_buffer.clear()
    for sound in &self.ctx.sounds:
      self.sound_buffer.push(sound.0 as u8)       ← [0] as bytes
      │
      ▼
Worker gameLoop():                               [engine.worker.ts:350]
    const soundLen = wasm.get_sound_events_len()  = 1
    const ptr = wasm.get_sound_events_ptr()
    const soundData = new Uint8Array(wasmMemory.buffer, ptr, 1)
    self.postMessage({ type: 'sound', events: [0] })
      │
      ▼
postMessage crosses thread boundary
      │
      ▼
useZapEngine worker.onmessage:                   [useZapEngine.ts:226]
    case 'sound':
      for (const id of e.data.events):
        soundManagerRef.current.play(id)          ← id = 0
          │
          ▼
SoundManager.play(0):                            [sound-manager.ts:87]
    resolveSound(0)                               ← lookup config.sounds[0]
      → { path: "click.mp3", volume: 1.0 }
    playBuffer("/audio/click.mp3", 1.0)
      │  const buffer = this.buffers.get(path)    ← pre-decoded AudioBuffer
      │  const source = this.ctx.createBufferSource()
      │  source.buffer = buffer
      │  source.connect(this.ctx.destination)      ← or through GainNode if volume < 1.0
      │  source.start()
      ▼
Audio plays through speakers
```

**Sounds always go through postMessage, never SAB.** The SAB has a sounds section but it's only used for the count — the actual event IDs are read from WASM memory and forwarded via postMessage because the main thread's SoundManager needs them as discrete events, not a shared buffer.

---

## 6. Resize Event Lifecycle

```
Browser window resizes or CSS layout changes
    │
    ▼
ResizeObserver fires                             [useZapEngine.ts:288]
    handleResize():
      │
      │  // Update canvas pixel dimensions (for devicePixelRatio)
      │  canvas.width = canvas.clientWidth * devicePixelRatio
      │  canvas.height = canvas.clientHeight * devicePixelRatio
      │
      │  // Tell renderer to recreate swapchain / resize framebuffers
      │  renderer.resize(canvas.width, canvas.height)
      │    → reconfigures GPUCanvasContext surface
      │    → recreates intermediate textures (scene, normal, bake)
      │    → rebuilds lighting bind groups (texture views changed)
      │
      │  // Tell worker the CSS dimensions (for coordinate conversion)
      │  worker.postMessage({
      │    type: 'resize',
      │    width: canvas.clientWidth,              ← CSS pixels, not device pixels
      │    height: canvas.clientHeight
      │  })
      │
      ▼
Worker onmessage:                                [engine.worker.ts:422]
    case 'resize':
      canvasWidth = e.data.width                   ← stored for screenToWorld()
      canvasHeight = e.data.height

      // Also forward visible world area to game for layout adaptation
      const proj = computeProjection(canvasWidth, canvasHeight, worldWidth, worldHeight)
      wasm.game_custom_event(99, proj.projWidth, proj.projHeight, 0)
        │
        ▼  kind=99 is the reserved "viewport resize" event
           game can use this to reposition UI elements, adjust camera, etc.
```

---

## 7. Initialization Sequence (Complete)

```
 React renders <App />
   │
   ▼
 useZapEngine hook effect runs                   [useZapEngine.ts:134]
   │
   ├─ 1. loadManifest(assetsUrl)                  ← fetch + JSON.parse assets.json
   │      returns AssetManifest { atlases, sprites, sounds }
   │
   ├─ 2. loadAssetBlobs(manifest, basePath)       ← fetch all atlas PNGs as Blobs
   │      returns Map<"tiles" → Blob, "chars" → Blob, ...>
   │
   ├─ 3. loadNormalMapBlobs(manifest, basePath)   ← fetch normal PNGs (optional)
   │      returns Map<"tiles" → Blob> (only atlases with normalMap field)
   │
   ├─ 4. createEngineWorker()                     ← new Worker('engine.worker.ts')
   │
   ├─ 5. SoundManager.init()                     ← pre-decode all audio buffers
   │      AudioContext created (suspended until first interaction)
   │
   ├─ 6. worker.postMessage({ type: 'init', wasmUrl, manifestJson })
   │      │
   │      ▼  [IN WORKER]
   │      import(wasmUrl)                          ← dynamic import of wasm-bindgen JS glue
   │      mod.default() → initResult               ← instantiate WASM module
   │      wasmMemory = initResult.memory
   │
   │      wasm.game_init()                         ← calls Rust: GameRunner::new() + init()
   │        Game::config()                         ← get GameConfig
   │        EngineContext::with_config()            ← allocate scene, physics, effects
   │        Game::init(&mut ctx)                   ← spawn initial entities
   │
   │      wasm.game_load_manifest(manifestJson)   ← JSON → SpriteRegistry
   │
   │      layout = ProtocolLayout.fromWasm(wasm)  ← read capacities via FFI
   │
   │      sharedBuffer = new SharedArrayBuffer(layout.bufferTotalBytes)
   │        ← e.g., ~800KB for default config
   │      Write capacities + protocol version into header
   │
   │      self.postMessage({
   │        type: 'ready',
   │        sharedBuffer,                          ← SAB transferred to main thread
   │        maxInstances, maxEffectsVertices, ...
   │        worldWidth, worldHeight
   │      })
   │
   │      running = true
   │      gameLoop()                               ← start game loop (setTimeout-based)
   │
   │      ▼  [BACK ON MAIN THREAD]
   │
   ├─ 7. worker.onmessage 'ready':
   │      sharedF32 = new Float32Array(e.data.sharedBuffer)
   │      layout = ProtocolLayout.fromHeader(sharedF32)
   │
   ├─ 8. initRenderer({
   │        canvas, manifest, atlasBlobs, normalMapBlobs,
   │        gameWidth, gameHeight, force2D,
   │        maxInstances, maxEffectsVertices, maxSdfInstances
   │      })
   │      │
   │      │  A. Probe WebGPU on disposable canvas
   │      │     ├─ success → create WebGPU renderer
   │      │     │    initDevice()                   ← adapter, device, surface config
   │      │     │    detect HDR tier
   │      │     │    loadAtlasTextures()             ← Blob → ImageBitmap → GPUTexture
   │      │     │    loadNormalTextures()
   │      │     │    createBuffers()                  ← instance, effects, SDF, vector, light
   │      │     │    createSpritePipelines()          ← one per atlas, ATLAS_COLS/ROWS overrides
   │      │     │    createEffectsPipeline()
   │      │     │    createSdfPipeline()
   │      │     │    createVectorPipeline()
   │      │     │    createLightingPipeline()
   │      │     │
   │      │     └─ failure → throw 'WebGPUInitFailed'
   │      │          hook catches → setCanvasKey(k+1) → React remounts canvas
   │      │          → re-runs effect with force2D=true
   │      │          → Canvas2D renderer created instead
   │      │
   │      ▼  returns Renderer object
   │
   ├─ 9. setIsReady(true)                         ← React renders game UI
   │
   └─ 10. startRenderLoop()                       ← requestAnimationFrame loop begins
```

---

## 8. What Crosses Each Boundary (Summary)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   WASM HEAP ──────── wasm-bindgen FFI ──────── WORKER JS                │
│                                                                         │
│   Direction: WASM → Worker (per frame)                                  │
│     Raw pointers: get_instances_ptr(), get_effects_ptr(), etc.          │
│     Scalar counts: get_instance_count(), get_light_count(), etc.        │
│     Scalar values: get_world_width(), get_ambient_r(), etc.             │
│     (~25 FFI calls per frame, each ~20ns = ~0.5μs total)                │
│                                                                         │
│   Direction: Worker → WASM (per frame + per input event)                │
│     game_tick(dt)                                                       │
│     game_pointer_down(x, y), game_key_down(keyCode), etc.               │
│     game_custom_event(kind, a, b, c)                                    │
│                                                                         │
│   Direction: Worker → WASM (once at init)                               │
│     game_init(), game_load_manifest(json)                               │
│                                                                         │
│   Data that NEVER crosses this boundary:                                │
│     Texture pixels, image data, GPU handles, DOM references             │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   WORKER JS ──────── SharedArrayBuffer ──────── MAIN THREAD JS          │
│                                                                         │
│   Direction: Worker → Main (per frame, via SAB memcpy)                  │
│     Instance data (8 floats × N instances)                              │
│     Effects vertices (5 floats × N vertices)                            │
│     SDF instances (12 floats × N instances)                             │
│     Vector vertices (6 floats × N vertices)                             │
│     Layer batches (4 floats × N batches)                                │
│     Light data (8 floats × N lights)                                    │
│     Header (28 floats: counts, world dims, timing, bake state)          │
│                                                                         │
│   Direction: Worker → Main (per frame, via postMessage)                 │
│     Sound event IDs: { type: 'sound', events: [0, 3, 1] }               │
│     Game events: { type: 'event', events: [{kind,a,b,c}, ...] }         │
│                                                                         │
│   Direction: Main → Worker (per event, via postMessage)                 │
│     Input: { type: 'pointer_down', x, y }                               │
│     Resize: { type: 'resize', width, height }                           │
│     Custom: { type: 'custom', kind, a, b, c }                           │
│     Lifecycle: { type: 'init' }, { type: 'stop' }, { type: 'resume' }   │
│                                                                         │
│   Data that NEVER crosses this boundary:                                │
│     GPU textures, shader modules, pipeline objects, audio buffers       │
│     (these are created and used exclusively on the main thread)         │
│                                                                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   MAIN THREAD JS ──── WebGPU API ──── GPU                               │
│                                                                         │
│   Direction: CPU → GPU (per frame)                                      │
│     device.queue.writeBuffer() for: instances, effects, SDF, vectors,   │
│       lights, camera projection matrix                                  │
│     command encoder → render passes → submit                            │
│                                                                         │
│   Direction: CPU → GPU (at init / on resize)                            │
│     Texture uploads (atlas images, normal maps)                         │
│     Pipeline creation (shader compilation)                              │
│     Framebuffer / render target creation                                │
│                                                                         │
│   Direction: GPU → Screen (implicit via present)                        │
│     Browser composites canvas content to display on vsync               │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```