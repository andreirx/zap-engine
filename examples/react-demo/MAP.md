# examples/react-demo/

Minimal React application demonstrating the `useZapEngine` hook with a HUD overlay.

## What It Shows

- `useZapEngine` hook managing the full engine lifecycle
- Canvas rendering with WebGPU → Canvas2D fallback
- FPS counter overlay using React state
- Canvas remount via `canvasKey` for WebGPU fallback recovery

## How to Run

From the project root:
```bash
npm run dev
# Then navigate to the react-demo entry in the Vite dev server
```

## Architecture Connection

Reuses the same WASM binary and assets from `examples/basic-demo/` — no new Rust code needed. Demonstrates the React integration layer at `src/engine/react/`.
