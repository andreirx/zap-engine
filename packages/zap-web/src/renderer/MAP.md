# renderer/

Dual-backend rendering system: WebGPU (HDR/EDR) with Canvas 2D fallback.

## Files

| File | Purpose |
|---|---|
| `types.ts` | `Renderer` interface — shared contract for both backends |
| `shaders.wgsl` | WGSL shaders: instanced sprites (vs_main/fs_main) + additive effects (vs_effects/fs_additive) |
| `webgpu.ts` | WebGPU backend: HDR/EDR progressive configure, manifest-driven N-atlas pipeline setup |
| `canvas2d.ts` | Canvas 2D fallback: manifest-driven rendering with heuristic glow approximation |
| `camera.ts` | Aspect-preserving orthographic projection math (parameterized, no hardcoded dimensions) |
| `index.ts` | Renderer factory: WebGPU probe + fallback strategy |

## Rendering Architecture

Two-pass rendering per frame:
1. **Alpha blend pass**: Sprites grouped by atlas (one pipeline per atlas with ATLAS_COLS/ROWS overrides)
2. **Additive blend pass**: Effects (electric arcs + particles) with procedural lightsaber glow profile, pushed into EDR range (×6.4)
