# react/

React integration layer for ZapEngine. Provides the `useZapEngine` hook that encapsulates the full engine lifecycle into a single hook call.

## Files

- **useZapEngine.ts** — The main hook. Manages worker, renderer (WebGPU -> Canvas2D fallback), SharedArrayBuffer reading, rAF render loop, input forwarding, resize, audio, and game events.
- **index.ts** — Public re-exports.

## Architecture Connection

This module imports from `../index` (the core engine) but is NOT exported by it. This keeps the core engine React-free. Consumers import via `@zap/web/react`:

```tsx
import { useZapEngine } from '@zap/web/react';
import type { GameEvent } from '@zap/web/react';
```

## Canvas Remount Pattern

When WebGPU initialization fails after touching the canvas (the `configure()` call taints it), the hook increments a `canvasKey` state counter. The consumer should use this as a React `key` prop on the `<canvas>` element, forcing React to unmount and remount a fresh DOM element.

```tsx
const { canvasRef, canvasKey } = useZapEngine({ ... });
return <canvas key={canvasKey} ref={canvasRef} />;
```
