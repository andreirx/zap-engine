# zap-engine

Core game engine library. Pure Rust — no wasm-bindgen, no wgpu, no platform dependencies.

## Module Layout

| Module | Purpose |
|---|---|
| `api/` | Public traits and types: `Game`, `GameConfig`, `EngineContext`, `RenderContext`, `EntityId`, `SoundEvent`, `GameEvent` |
| `core/` | Engine internals: `Scene` (entity storage), `FixedTimestep` |
| `components/` | Data structs: `Entity` (fat entity), `SpriteComponent`, placeholders for `Emitter` and `Mesh` |
| `systems/` | Engine systems: render buffer builder, effects (electric arcs, particles) |
| `renderer/` | Render data types (NO GPU code): `RenderInstance`, `RenderBuffer`, `Camera2D` |
| `bridge/` | SharedArrayBuffer layout constants (mirrors TypeScript `protocol.ts`) |
| `input/` | Input event queue: `InputEvent`, `InputQueue` |
| `assets/` | Asset manifest types: `AssetManifest`, `AtlasDescriptor`, `SpriteDescriptor` |

## Design Principles

- Zero platform dependencies — compiles for native and wasm32 targets
- Data-oriented: flat Vec<Entity> storage, no trait objects in hot paths
- All public APIs use SharedArrayBuffer-compatible repr(C) layouts
