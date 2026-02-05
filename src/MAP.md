# src/

TypeScript engine — the browser-side runtime for ZapEngine.

All rendering, audio, and worker management happens here. The Rust/WASM side
produces flat render buffers; this code reads them and draws to the screen.

## Subdirectories

| Directory | Purpose |
|---|---|
| `engine/` | Core TypeScript engine: renderer, worker, assets, audio |
