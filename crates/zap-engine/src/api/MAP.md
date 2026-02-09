# api/

Public API surface for game developers. This module exposes the core types games interact with.

## Files

| File | Purpose |
|------|---------|
| `game.rs` | `EngineContext` facade, `GameConfig`, `BakeState`, `Game` trait |
| `types.rs` | Type aliases (`EntityId`, `BodyHandle`, etc.) |

## Key Types

- **`Game` trait**: Games implement `init()` and `update()` to drive logic
- **`EngineContext`**: Facade providing access to all engine subsystems (scene, effects, physics, vectors, lights, sounds)
- **`GameConfig`**: Configurable capacities and world dimensions passed at init
- **`BakeState`**: Tracks which render layers are baked (cached as textures)

## Architecture Notes

`EngineContext` follows the facade pattern, exposing all subsystems (`scene`, `effects`, `physics`, `vectors`, `lights`) as public fields. Convenience methods like `spawn_with_body()` and `despawn()` coordinate multiple subsystems atomically.

The `with_config(&GameConfig)` constructor flows configuration down to all subsystems (Scene capacity, Effects seed, etc.).
