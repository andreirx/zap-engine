# packages/

NPM packages that form the TypeScript side of ZapEngine.

| Package | NPM Name | Purpose |
|---|---|---|
| `zap-web/` | `@zap/web` | TypeScript engine runtime: renderer (WebGPU + Canvas2D), worker, assets, audio, React hook |

## Consuming

Games import via package names (resolved by Vite aliases in root config):

```tsx
import { initRenderer, createEngineWorker } from '@zap/web';
import { useZapEngine } from '@zap/web/react';
```

No build step needed — Vite resolves the `exports` field directly to `.ts` source files.
