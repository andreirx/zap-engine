# engine/

TypeScript engine runtime. Reads simulation output from Rust/WASM and renders it.

## Subdirectories

| Directory | Purpose |
|---|---|
| `renderer/` | WebGPU + Canvas 2D rendering backends, shader code, camera math |
| `worker/` | Web Worker management, SharedArrayBuffer protocol |
| `assets/` | Asset manifest types, blob fetching, GPU texture creation |
| `audio/` | Web Audio API sound manager |

## Data Flow

```
Game (Rust) → RenderBuffer (SharedArrayBuffer) → Worker → Renderer (WebGPU/Canvas2D) → Screen
                                                        → SoundManager → Speakers
```
