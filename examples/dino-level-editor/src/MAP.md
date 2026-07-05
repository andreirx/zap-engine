# examples/dino-level-editor/src/

Rust/WASM implementation of the dinosaur level editor example.

| File | Purpose |
|---|---|
| `lib.rs` | Invokes `zap_web::export_game!` to expose the standard worker-facing WASM API |
| `game.rs` | Example-local editor/play state machine, terrain rules, player movement, money collection, volcano/fireball hazard, camera transform, and render synchronization |

## Under the Hood

The renderer currently projects a fixed game coordinate plane and does not consume `EngineContext::camera` through the SharedArrayBuffer protocol. This example therefore performs camera conversion inside `game.rs`: logical level coordinates are transformed into the fixed 1600×900 render plane before entities are spawned. That keeps the engine boundary stable while still demonstrating a zooming editor/play camera.
