# core/

Core runtime structures: entity storage, physics world, and timing.

## Files

| File | Purpose |
|------|---------|
| `scene.rs` | `Scene` — entity container with spawn/despawn/find |
| `physics.rs` | `PhysicsWorld` — rapier2d wrapper (feature-gated) |
| `time.rs` | `FixedTimestep` for deterministic physics |

## Key Types

- **`Scene`**: Flat `Vec<Entity>` storage with ID recycling, tag-based queries
- **`PhysicsWorld`**: Wraps all 9 rapier2d structs (bodies, colliders, joints, etc.)
- **`FixedTimestep`**: Accumulator-based timing (default 60Hz) with leftover interpolation

## Architecture Notes

`Scene` uses a simple Vec with inactive slot recycling. `EntityId` is a unique monotonic counter — IDs are never reused.

`PhysicsWorld` hides all nalgebra types behind a `glam::Vec2` API. It stores `EntityId` in rapier's `user_data` field to map collisions back to game entities.

The physics feature is default-on but can be disabled for games that don't need rigid bodies (e.g., zapzap-mini).

## Game Loop Order

```
1. game.update()       — input handling, spawn/despawn
2. ctx.step_physics()  — rapier step + position sync
3. tick_emitters()     — particle spawning
4. effects.tick(dt)    — particle/arc updates
```
