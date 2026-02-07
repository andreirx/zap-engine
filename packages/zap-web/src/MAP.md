# @zap/web — TypeScript Engine Runtime

NPM package `@zap/web`. Reads simulation output from Rust/WASM and renders it.

## Entry Points

| Import | File | Purpose |
|---|---|---|
| `@zap/web` | `index.ts` | Core: renderer, assets, audio, protocol constants, `createEngineWorker()` |
| `@zap/web/react` | `react/index.ts` | React hook: `useZapEngine`, types |

## Subdirectories

| Directory | Purpose |
|---|---|
| `renderer/` | WebGPU + Canvas 2D rendering backends, WGSL shaders, camera math, layer compositor |
| `worker/` | Web Worker management, SharedArrayBuffer protocol |
| `assets/` | Asset manifest types, blob fetching, GPU texture creation, normal map loading |
| `audio/` | Web Audio API sound manager |
| `react/` | `useZapEngine` hook — full engine lifecycle in a single hook call |

## Data Flow

```
Game (Rust) → RenderBuffer (SharedArrayBuffer) → Worker → Renderer (WebGPU/Canvas2D) → Screen
                                                        → SoundManager → Speakers
```
