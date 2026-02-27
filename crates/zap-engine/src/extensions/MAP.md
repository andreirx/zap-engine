# extensions/

Optional extension systems for ZapEngine. These are decoupled from core Entity/Scene — games opt-in by creating these systems alongside their Scene.

**Design principle**: Core stays simple, complexity is additive.

## Files

| File | Purpose |
|------|---------|
| `mod.rs` | Module exports |
| `easing.rs` | Pure easing functions (no state, no dependencies) |
| `transform.rs` | Transform hierarchy — parent-child relationships by EntityId |
| `tween.rs` | Tween system — animated value transitions by EntityId |

## Architecture

All extensions operate on `EntityId` as keys, never on Entity references directly. This maintains clean boundaries:

```
┌──────────────────┐     ┌──────────────────┐
│      Scene       │     │   Extensions     │
│  (entity storage)│     │  (by EntityId)   │
├──────────────────┤     ├──────────────────┤
│ Vec<Entity>      │◄────│ TransformGraph   │
│                  │     │ TweenState       │
└──────────────────┘     └──────────────────┘
```

## Usage Examples

### Easing (Pure Math)

```rust
use zap_engine::{Easing, ease, ease_vec2};

// Apply easing to a value
let t = 0.5;  // progress [0, 1]
let eased = Easing::QuadOut.apply(t);  // faster start, slower end

// Interpolate with easing
let value = ease(0.0, 100.0, t, Easing::ElasticOut);
let pos = ease_vec2(Vec2::ZERO, Vec2::new(100.0, 50.0), t, Easing::BackOut);
```

### Transform Hierarchy

```rust
use zap_engine::{TransformGraph, LocalTransform, EntityId};

let mut graph = TransformGraph::new();

// Register entities
graph.register_with(parent_id, LocalTransform::new().with_offset(Vec2::new(100.0, 100.0)));
graph.register_with(child_id, LocalTransform::new().with_offset(Vec2::new(50.0, 0.0)));

// Establish hierarchy
graph.set_parent(child_id, Some(parent_id));

// In game loop — propagate transforms to Scene
graph.propagate(&mut scene);
```

### Tweens

```rust
use zap_engine::{TweenState, Tween, Easing, TweenLoop};

let mut tweens = TweenState::new();

// Simple position tween
tweens.add(entity_id, Tween::position(
    Vec2::ZERO,
    Vec2::new(200.0, 100.0),
    0.5,  // duration
    Easing::QuadOut,
));

// Looping scale pulse
tweens.add(entity_id, Tween::scale_uniform(1.0, 1.2, 0.3, Easing::SineInOut)
    .with_loop(TweenLoop::PingPong));

// Fade out with completion callback
tweens.add(entity_id, Tween::fade_out(0.5, Easing::Linear)
    .with_on_complete(42));  // emits event ID 42

// In game loop
tweens.tick(dt, &mut scene);

// Handle completions
for event_id in tweens.drain_completed() {
    // Emit as GameEvent or handle internally
}
```

## Available Easings

| Category | Variants |
|----------|----------|
| Linear | `Linear` |
| Quadratic | `QuadIn`, `QuadOut`, `QuadInOut` |
| Cubic | `CubicIn`, `CubicOut`, `CubicInOut` |
| Quartic | `QuartIn`, `QuartOut`, `QuartInOut` |
| Sine | `SineIn`, `SineOut`, `SineInOut` |
| Exponential | `ExpoIn`, `ExpoOut`, `ExpoInOut` |
| Back (overshoot) | `BackIn`, `BackOut`, `BackInOut` |
| Bounce | `BounceOut` |
| Elastic | `ElasticOut` |

## Tween Targets

| Target | What It Animates |
|--------|------------------|
| `Position` | Entity.pos (Vec2) |
| `PositionX` | Entity.pos.x |
| `PositionY` | Entity.pos.y |
| `Rotation` | Entity.rotation |
| `Scale` | Entity.scale (Vec2) |
| `ScaleX` | Entity.scale.x |
| `ScaleY` | Entity.scale.y |
| `Alpha` | Entity.sprite.alpha |

## Loop Modes

| Mode | Behavior |
|------|----------|
| `Once` | Play once, then remove tween |
| `Loop` | Restart from beginning |
| `PingPong` | Reverse direction on completion |
