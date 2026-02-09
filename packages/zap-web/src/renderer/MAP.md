# renderer/

Dual-backend rendering system: WebGPU (HDR/EDR) with Canvas 2D fallback.

## Directory Structure

```
renderer/
├── types.ts              # Renderer interface — shared contract
├── index.ts              # Renderer factory: WebGPU probe + fallback
├── webgpu.ts             # WebGPU facade (orchestration only)
├── webgpu/               # Modular WebGPU implementation
│   ├── device.ts         # Device init, tier detection, context config
│   ├── resources.ts      # Textures, buffers, bind groups
│   ├── pipelines/        # Pipeline definitions
│   │   ├── common.ts     # Shared layouts and blend targets
│   │   ├── sprite.ts     # Sprite + normal pipelines
│   │   ├── effects.ts    # Additive effects pipeline
│   │   ├── sdf.ts        # SDF molecule pipeline
│   │   ├── vector.ts     # Vector geometry pipeline
│   │   └── lighting.ts   # Post-process lighting pipeline
│   └── passes/           # Render pass encoding
│       ├── bake.ts       # Layer baking pass
│       └── scene.ts      # Main scene + lighting composition
├── canvas2d.ts           # Canvas 2D fallback
├── camera.ts             # Orthographic projection math
├── compositor.ts         # Layer bake caching
├── constants.ts          # Segment colors, GPU utilities
└── *.wgsl                # WGSL shaders
```

## Module Responsibilities

### webgpu.ts (Facade)
Orchestrates initialization and the per-frame render loop. Imports from submodules and wires them together. Preserves the original public API (`initWebGPURenderer` → `Renderer`).

### webgpu/device.ts
- Request GPU adapter/device
- Progressive HDR/EDR context configuration
- `RenderTier` detection and `GLOW_MULT` constants
- `resizeContext()` for window resize handling

### webgpu/resources.ts
- Atlas and normal map texture loading
- Buffer creation (camera, instances, effects, SDF, vectors, lights)
- Bind group layout and bind group creation
- Fallback textures (1x1 white, flat normal)

### webgpu/pipelines/
Each module exports a `create*Pipeline()` function:
- **common.ts**: Shared pipeline layouts, blend target configurations
- **sprite.ts**: Alpha-blend sprite pipelines (one per atlas) + normal pipelines
- **effects.ts**: Additive blend for particles and electric arcs
- **sdf.ts**: Raymarched molecules (sphere, capsule, rounded box)
- **vector.ts**: CPU-tessellated polygons from lyon
- **lighting.ts**: Fullscreen post-process with normal map support

### webgpu/passes/
Render pass encoding logic:
- **bake.ts**: Renders dirty baked layers to intermediate textures
- **scene.ts**: Main render pass (sprites → vectors → SDF → effects), normal pass, lighting pass

## Shaders

| File | Purpose |
|------|---------|
| `shaders.wgsl` | Instanced sprites (vs_main/fs_main), effects (vs_effects/fs_additive), normals (fs_normal) |
| `molecule.wgsl` | SDF raymarching with Phong + Fresnel + HDR emissive |
| `vector.wgsl` | Vector/polygon rendering |
| `lighting.wgsl` | Dynamic lighting post-process with normal map sampling |
| `composite.wgsl` | Layer bake compositing |

## Rendering Pipeline

Multi-pass rendering per frame:

1. **Bake Pass** (conditional): Render dirty baked layers to offscreen textures
2. **Scene Pass**:
   - Sprites sorted by (layer, atlas), with baked layers blitted from cache
   - Vectors (alpha blend)
   - SDF molecules (alpha blend)
   - Effects (additive blend)
3. **Normal Pass** (when lighting + normal maps): Render sprite normals to deferred buffer
4. **Lighting Pass** (when active): Fullscreen post-process compositing point lights

## Extensibility

Adding a new render feature (e.g., Bloom):
1. Create `webgpu/pipelines/bloom.ts` with `createBloomPipeline()`
2. Create `webgpu/passes/bloom.ts` with `encodeBloomPass()`
3. Wire into `webgpu.ts` facade

The modular structure keeps each concern isolated and testable.
