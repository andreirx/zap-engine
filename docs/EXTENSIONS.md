# Extensions

Optional extension systems for ZapEngine. These are **decoupled from core Entity/Scene** — games opt-in by creating these systems alongside their Scene.

**Design principle**: Core stays simple, complexity is additive.

---

## Architecture

Extensions operate on `EntityId` as keys, never on Entity references directly. This maintains clean boundaries:

```
┌──────────────────────────────────────────────────────────────────┐
│                        Game Code                                  │
│                                                                   │
│   ┌──────────────┐     ┌──────────────┐     ┌──────────────┐    │
│   │    Scene     │     │ TweenState   │     │TransformGraph│    │
│   │   (core)     │◄────│ (extension)  │     │ (extension)  │    │
│   │              │     │              │     │              │    │
│   │ Vec<Entity>  │◄────────────────────────│ by EntityId  │    │
│   └──────────────┘     └──────────────┘     └──────────────┘    │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

**Key benefits:**
- No modifications to Entity or Scene structs
- Games only pay for features they use
- Extensions can be composed independently
- Easy to test in isolation

---

## Quick Start

```rust
use zap_engine::*;

struct MyGame {
    tweens: TweenState,
    transforms: TransformGraph,
}

impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) {
        // Spawn entities normally
        let parent = EntityId(1);
        let child = EntityId(2);
        ctx.spawn(Entity::new(parent).with_pos(Vec2::new(100.0, 100.0)));
        ctx.spawn(Entity::new(child));

        // Set up hierarchy (extension)
        self.transforms.register(parent);
        self.transforms.register_with(child,
            LocalTransform::new().with_offset(Vec2::new(50.0, 0.0)));
        self.transforms.set_parent(child, Some(parent));

        // Add a tween (extension)
        self.tweens.add(parent, Tween::position(
            Vec2::new(100.0, 100.0),
            Vec2::new(500.0, 300.0),
            2.0,
            Easing::QuadOut,
        ));
    }

    fn update(&mut self, ctx: &mut EngineContext, dt: f32) {
        // Tick tweens — modifies entity positions/rotations/etc in scene
        self.tweens.tick(dt, &mut ctx.scene);

        // Propagate hierarchy — child world pos = parent pos + local offset
        self.transforms.propagate(&mut ctx.scene);
    }
}
```

---

## Easing Functions

Pure math functions for smooth interpolation. No state, no dependencies.

### Available Easings

| Category | Variants | Character |
|----------|----------|-----------|
| Linear | `Linear` | Constant velocity |
| Quadratic | `QuadIn`, `QuadOut`, `QuadInOut` | Gentle acceleration |
| Cubic | `CubicIn`, `CubicOut`, `CubicInOut` | Stronger acceleration |
| Quartic | `QuartIn`, `QuartOut`, `QuartInOut` | Very strong acceleration |
| Sine | `SineIn`, `SineOut`, `SineInOut` | Smooth, natural |
| Exponential | `ExpoIn`, `ExpoOut`, `ExpoInOut` | Dramatic, punchy |
| Back | `BackIn`, `BackOut`, `BackInOut` | Overshoot then settle |
| Bounce | `BounceOut` | Bouncy finish |
| Elastic | `ElasticOut` | Spring-like wobble |

### Usage

```rust
use zap_engine::{Easing, ease, ease_vec2, lerp};

// Apply easing to normalized time [0, 1]
let t = 0.5;
let eased_t = Easing::QuadOut.apply(t);  // 0.75 (faster start)

// Interpolate values with easing
let value = ease(0.0, 100.0, t, Easing::QuadOut);  // 75.0
let pos = ease_vec2(Vec2::ZERO, Vec2::new(100.0, 50.0), t, Easing::BackOut);

// Simple linear interpolation
let mid = lerp(0.0, 100.0, 0.5);  // 50.0
```

### Easing Curves

```
Linear:     ────────────────    (constant)
QuadOut:    ══════──────────    (fast start, slow end)
QuadIn:     ──────══════════    (slow start, fast end)
BackOut:    ══════════╮─────    (overshoot then settle)
ElasticOut: ═══╭─╮─╭────────    (spring wobble)
BounceOut:  ═══╮╭╮╭─────────    (bouncy)
```

---

## Transform Hierarchy

Parent-child relationships tracked by EntityId. Moving a parent automatically moves all children.

### API

```rust
use zap_engine::{TransformGraph, LocalTransform, EntityId};

let mut graph = TransformGraph::new();

// Register entities
graph.register(parent_id);
graph.register_with(child_id, LocalTransform::new()
    .with_offset(Vec2::new(50.0, 0.0))    // 50px right of parent
    .with_rotation(0.0)                     // no additional rotation
    .with_scale(Vec2::ONE));                // same scale as parent

// Establish hierarchy
graph.set_parent(child_id, Some(parent_id));

// Query relationships
let parent = graph.get_parent(child_id);           // Some(parent_id)
let children = graph.get_children(parent_id);      // &[child_id]

// Modify local transform
if let Some(local) = graph.get_local_mut(child_id) {
    local.offset.x = 100.0;  // Now 100px right of parent
}

// Remove from hierarchy (children become roots)
graph.remove(parent_id);

// In game loop — apply hierarchy to scene
graph.propagate(&mut scene);
```

### Transform Propagation

When `propagate()` is called:

1. World position = parent world position + (local offset rotated by parent rotation) × parent scale
2. World rotation = parent rotation + local rotation
3. World scale = parent scale × local scale

```
Parent at (100, 100), rotation 90°
Child local offset (50, 0)

After propagate:
  Child world pos = (100, 100) + rotate((50, 0), 90°) = (100, 150)
```

### Best Practices

- Call `propagate()` **after** tweens but **before** rendering
- Only register entities that participate in hierarchy
- Use `mark_dirty()` if you modify entity positions directly and need re-propagation

---

## Tween System

Animated value transitions. Tweens modify entity properties over time.

### Creating Tweens

```rust
use zap_engine::{TweenState, Tween, Easing, TweenLoop};

let mut tweens = TweenState::new();

// Position tween
tweens.add(entity_id, Tween::position(
    Vec2::new(0.0, 0.0),      // from
    Vec2::new(200.0, 100.0),  // to
    0.5,                       // duration (seconds)
    Easing::QuadOut,
));

// Single-axis position
tweens.add(entity_id, Tween::position_x(0.0, 100.0, 0.3, Easing::Linear));
tweens.add(entity_id, Tween::position_y(0.0, 50.0, 0.3, Easing::Linear));

// Rotation (radians)
tweens.add(entity_id, Tween::rotation(0.0, std::f32::consts::PI, 1.0, Easing::SineInOut));

// Scale
tweens.add(entity_id, Tween::scale(
    Vec2::ONE,
    Vec2::new(2.0, 2.0),
    0.5,
    Easing::BackOut,
));
tweens.add(entity_id, Tween::scale_uniform(1.0, 1.5, 0.3, Easing::QuadOut));

// Alpha (fade)
tweens.add(entity_id, Tween::alpha(1.0, 0.0, 0.5, Easing::Linear));
tweens.add(entity_id, Tween::fade_in(0.3, Easing::QuadOut));
tweens.add(entity_id, Tween::fade_out(0.3, Easing::QuadIn));
```

### Loop Modes

```rust
// Play once and remove (default)
Tween::position(...).with_loop(TweenLoop::Once)

// Restart from beginning
Tween::scale_uniform(1.0, 1.2, 0.3, Easing::SineInOut)
    .with_loop(TweenLoop::Loop)

// Ping-pong (reverse on completion)
Tween::position_y(0.0, 50.0, 0.5, Easing::QuadInOut)
    .with_loop(TweenLoop::PingPong)
```

### Completion Callbacks

```rust
// Emit event ID when tween completes
let tween_id = tweens.add(entity_id,
    Tween::fade_out(0.5, Easing::Linear)
        .with_on_complete(42)  // event ID
);

// In update loop
tweens.tick(dt, &mut scene);

// Handle completions
for event_id in tweens.drain_completed() {
    match event_id {
        42 => { /* fade out finished, despawn entity? */ }
        _ => {}
    }
}
```

### Controlling Tweens

```rust
// Get handle when adding
let handle: TweenId = tweens.add(entity_id, tween);

// Pause/resume
tweens.pause(handle);
tweens.resume(handle);

// Pause/resume all
tweens.pause_all();
tweens.resume_all();

// Remove specific tween
tweens.remove(handle);

// Remove all tweens for an entity
tweens.remove_entity(entity_id);

// Query
let count = tweens.len();
let is_empty = tweens.is_empty();
```

### Tween Targets

| Target | What It Animates |
|--------|------------------|
| `Position` | `entity.pos` (Vec2) |
| `PositionX` | `entity.pos.x` |
| `PositionY` | `entity.pos.y` |
| `Rotation` | `entity.rotation` |
| `Scale` | `entity.scale` (Vec2) |
| `ScaleX` | `entity.scale.x` |
| `ScaleY` | `entity.scale.y` |
| `Alpha` | `entity.sprite.alpha` (requires sprite) |

---

## Combining Extensions

Extensions compose naturally:

```rust
struct MyGame {
    tweens: TweenState,
    transforms: TransformGraph,
}

impl Game for MyGame {
    fn update(&mut self, ctx: &mut EngineContext, dt: f32) {
        // 1. Update game logic (spawns, despawns, state changes)

        // 2. Tick tweens (modifies entity.pos, etc.)
        self.tweens.tick(dt, &mut ctx.scene);

        // 3. Propagate hierarchy (child positions relative to parents)
        self.transforms.propagate(&mut ctx.scene);

        // 4. Physics step (if using physics)
        ctx.step_physics(dt);

        // 5. Effects, particles, etc.
    }
}
```

**Order matters:**
- Tweens before transforms → tweened parents affect children
- Transforms before physics → hierarchy applied before collision detection
- Physics after all position changes → accurate collision response

---

## Migration Guide

Existing games require **zero changes**. Extensions are purely additive:

| Before | After |
|--------|-------|
| `Entity::new(id).with_pos(pos)` | Same — no change |
| `ctx.spawn(entity)` | Same — no change |
| Manual lerp in update | Optional: use `TweenState` |
| Flat entity structure | Optional: use `TransformGraph` |

To adopt extensions:
1. Add `TweenState` / `TransformGraph` to your game struct
2. Call `tick()` / `propagate()` in your update loop
3. Replace manual interpolation with `Tween::*` calls
