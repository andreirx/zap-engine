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

Layout: `[Header (12 floats)] [Instances (512×8 floats)] [Effects (16384×5 floats)] [Sounds (32×1)] [Events (32×4)]`
