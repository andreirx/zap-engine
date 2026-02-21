# The Coordinate Space Contract

## The Lesson

**Any ZapEngine game has ONE source of truth for its coordinate space: `GameConfig`.**

```rust
fn config(&self) -> GameConfig {
    GameConfig {
        world_width: 1600.0,   // ← THE source of truth
        world_height: 900.0,   // ← THE source of truth
        ..Default::default()
    }
}
```

Everything else — the worker, the renderer, the React hook, the projection matrix — MUST derive from this single source. The moment you have two places defining dimensions, you have a bug waiting to happen.

---

## What Went Wrong (The Solar System Bug)

The pipeline has 5 layers, and coordinate dimensions were specified (or defaulted) at each:

```
┌─────────────────────────────────────────────────────────────────────────┐
│  LAYER              │  DIMENSION SOURCE           │  VALUE              │
├─────────────────────┼─────────────────────────────┼─────────────────────┤
│  1. Game (Rust)     │  GameConfig                 │  1600 × 900  ✓      │
│  2. Worker (TS)     │  get_world_width/height()   │  1600 × 900  ✓      │
│  3. Hook config     │  gameWidth/gameHeight prop  │  (not provided)     │
│  4. Hook default    │  destructuring default      │  800 × 600   ✗ BUG! │
│  5. Renderer        │  from hook                  │  800 × 600   ✗      │
└─────────────────────────────────────────────────────────────────────────┘
```

The hook had `gameWidth = 800` as a destructuring default. When the app didn't provide this prop, the default won, and the worker's correct value was never used.

**Result:** Renderer thought center was (400, 300). Game thought center was (800, 450). Sun appeared in bottom-right.

---

## The Contract

### Rule 1: GameConfig is Canonical

Your game's `config()` method defines the world. Period.

```rust
// This is the ONLY place world dimensions should be defined
fn config(&self) -> GameConfig {
    GameConfig {
        world_width: YOUR_WIDTH,
        world_height: YOUR_HEIGHT,
        // ...
    }
}
```

### Rule 2: Never Override at the React Layer

**DO NOT** pass `gameWidth`/`gameHeight` to `useZapEngine`:

```tsx
// ✗ BAD — creates a second source of truth
useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    gameWidth: 1600,   // ← Don't do this
    gameHeight: 900,   // ← The game already knows its size
});

// ✓ GOOD — let the engine derive from GameConfig
useZapEngine({
    wasmUrl: WASM_URL,
    assetsUrl: ASSETS_URL,
    // No gameWidth/gameHeight — engine reads from WASM
});
```

The only exception: if you're building a generic game host that doesn't know the game's dimensions ahead of time.

### Rule 3: Understand the Coordinate Flow

```
GameConfig (Rust)
    │
    ▼ game_init() stores config
    │
get_world_width() / get_world_height()
    │
    ▼ Worker reads after WASM init
    │
'ready' message { worldWidth, worldHeight }
    │
    ▼ useZapEngine receives
    │
initRenderer({ gameWidth, gameHeight })
    │
    ▼ buildProjectionMatrix()
    │
Projection maps [0, projWidth] × [0, projHeight] to NDC
    │
    ▼ computeProjection() extends to match canvas aspect
    │
CUSTOM_RESIZE event sent to game with projWidth/projHeight
    │
    ▼ Game updates visible_w/visible_h
    │
Game's screen_center() = (visible_w/2, visible_h/2)
```

If ANY step in this chain uses wrong or default values, your coordinate spaces diverge.

### Rule 4: Visible Dimensions ≠ World Dimensions

```
world_width/height  = The game's design-time coordinate space (e.g., 1600×900)
projWidth/Height    = Extended to match canvas aspect ratio
visible_w/h         = What the game sees after aspect correction
```

Example: 1600×900 game on a 4:3 monitor
- `world_width = 1600, world_height = 900`
- `projWidth = 1600, projHeight = 1200` (extended vertically)
- `visible_w = 1600, visible_h = 1200`

**Your game must handle `visible_w/h` potentially being larger than `world_width/height`.**

### Rule 5: Camera Center is Dynamic

The center of the viewport is NOT `(world_width/2, world_height/2)`.

It's `(visible_w/2, visible_h/2)` — which changes based on the user's screen aspect ratio.

```rust
// ✗ WRONG — hardcoded center
let center_x = WORLD_W / 2.0;

// ✓ RIGHT — dynamic center
let center_x = self.visible_w / 2.0;
```

---

## Debugging Checklist

When objects appear offset from where they should be:

1. **Log the pipeline values:**
   ```rust
   // In game's update():
   log::info!("visible: {}×{}", self.visible_w, self.visible_h);
   ```
   ```typescript
   // In worker 'ready' handler:
   console.log('worldWidth:', worldWidth, 'worldHeight:', worldHeight);
   ```
   ```typescript
   // In useZapEngine:
   console.log('renderer dims:', rendererGameWidth, rendererGameHeight);
   ```

2. **Check for divergence:** If any layer has different values, that's your bug.

3. **Check CUSTOM_RESIZE:** Is the game receiving and processing the resize event? Add logging to your `InputEvent::Custom { kind: 99, .. }` handler.

4. **Check timing:** Does the first frame render before CUSTOM_RESIZE is processed? The worker now calls `game_tick(0)` after sending resize, but verify this is happening.

---

## Summary

| Principle | Implementation |
|-----------|----------------|
| Single Source of Truth | `GameConfig` in Rust |
| No Silent Defaults | Hook uses `undefined`, not `800` |
| Explicit Propagation | Worker sends `worldWidth/Height` in 'ready' |
| Dynamic Adaptation | Game receives `projWidth/Height` via CUSTOM_RESIZE |
| Aspect-Aware Center | `(visible_w/2, visible_h/2)`, not hardcoded |

**The coordinate space is a contract between your game and the engine. Break the contract, and objects end up in the wrong place.**
