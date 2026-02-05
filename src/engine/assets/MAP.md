# assets/

Asset manifest and loading system. Drives the renderer with a JSON-based configuration
instead of hardcoded atlas layouts.

## Files

| File | Purpose |
|---|---|
| `manifest.ts` | `AssetManifest` TypeScript type + `loadManifest(url)` function |
| `loader.ts` | Blob fetching + GPU texture / HTMLImage creation |

## Asset Pipeline

1. Game provides `assets.json` manifest listing atlases and named sprites
2. `loadManifest()` fetches and parses the manifest
3. `loadAssetBlobs()` fetches all atlas PNGs as Blobs
4. Renderer-specific functions create GPU textures or canvas images from blobs
