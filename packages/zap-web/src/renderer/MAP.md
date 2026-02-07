# renderer/

Dual-backend rendering system: WebGPU (HDR/EDR) with Canvas 2D fallback.

## Files

| File | Purpose |
|---|---|
| `types.ts` | `Renderer` interface — shared contract for both backends |
| `index.ts` | Renderer factory: WebGPU probe + fallback strategy |
| `webgpu.ts` | WebGPU backend: HDR/EDR progressive configure, manifest-driven N-atlas pipeline, layer batches, dynamic lighting, normal maps |
| `canvas2d.ts` | Canvas 2D fallback: manifest-driven rendering with heuristic glow approximation |
| `camera.ts` | Aspect-preserving orthographic projection math (parameterized, no hardcoded dimensions) |
| `compositor.ts` | `LayerCompositor` — per-layer GPUTexture render targets for baked layer caching |
| `constants.ts` | Segment colors, GPU color packing utilities |
| `shaders.wgsl` | WGSL shaders: instanced sprites (vs_main/fs_main), additive effects (vs_effects/fs_additive), normal pass (fs_normal) |
| `molecule.wgsl` | SDF raymarching shader: sphere, capsule, rounded box with Phong + Fresnel shading |
| `vector.wgsl` | Vector/polygon rendering shader |
| `lighting.wgsl` | Dynamic lighting post-process: fullscreen triangle, quadratic falloff, normal map sampling |
| `composite.wgsl` | Layer bake compositing shader |

## Rendering Architecture

Multi-pass rendering per frame:
1. **Alpha blend pass**: Sprites grouped by atlas (one pipeline per atlas with ATLAS_COLS/ROWS overrides), sorted by render layer
2. **Vector pass**: CPU-tessellated polygons/polylines via lyon
3. **SDF pass**: Raymarched molecules (spheres, capsules, rounded boxes)
4. **Additive blend pass**: Effects (electric arcs + particles) with procedural lightsaber glow profile, pushed into EDR range
5. **Lighting pass** (when active): Fullscreen post-process compositing point lights with normal map support
6. **Layer baking**: Static layers rendered once to offscreen textures, composited each frame
