# worker/

Web Worker management for off-main-thread game simulation.

## Files

| File | Purpose |
|---|---|
| `protocol.ts` | SharedArrayBuffer layout constants (mirrors Rust `bridge/protocol.rs`) |
| `engine.worker.ts` | Generic worker: loads WASM, runs game loop, writes to SharedArrayBuffer |

## Protocol

The worker communicates with the main thread via:
1. **SharedArrayBuffer** (preferred): Zero-copy reads from main thread using Atomics
2. **postMessage fallback**: Buffer copies when COOP/COEP headers are absent

Layout: `[Header (28 floats)] [Instances] [Effects] [Sounds] [Events] [SDF] [Vectors] [LayerBatches] [Lights]`

Protocol version: 4.0
