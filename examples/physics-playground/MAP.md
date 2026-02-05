# examples/physics-playground/

Angry Birds-style physics sandbox demonstrating sprites, physics, collision effects, and React UI integration.

## Gameplay

- **Sling**: Drag near the left sling point (200, 450) to aim and release to launch a projectile
- **Tower**: 5x3 grid of colored blocks stacked on the right side
- **Score**: Blocks knocked over (rotated or fallen) count toward the score
- **Reset**: Button sends a custom event to restart the level

## Engine Features Demonstrated

- **Physics** (Rapier2D): Dynamic blocks with cuboid colliders, ball projectile with CCD
- **Custom Events** (A4): React Reset button → `InputEvent::Custom { kind: 1 }` → game reset
- **World Coordinates** (A2): Pointer events arrive in world coords, sling drag works correctly
- **Collision Sparks**: Particle effects spawned at collision midpoints
- **Sling Band**: Electric arc effect drawn between sling origin and drag point during aiming
- **Game Events**: Score updates sent to React via `GameEvent { kind: 1.0, a: score }`

## Architecture

```
src/lib.rs      — WASM exports (thread_local! GameRunner pattern)
src/game.rs     — PhysicsPlayground: Game trait impl, state machine (Aiming→Flying→Settled)
App.tsx         — React UI with score display, reset button, FPS counter
main.tsx        — React entry point
```

## How to Run

1. Build WASM: `wasm-pack build examples/physics-playground --target web --out-dir pkg`
2. From project root: `npm run dev` → navigate to the playground entry
