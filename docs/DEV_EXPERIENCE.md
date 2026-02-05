This document defines the **Developer Experience (DX)**. It serves as the style guide for both the engine maintainers and the game developers building on top of it.

---

# DEVELOPER_GUIDE.md

## 1. The Mental Model

To write clean code in ZapEngine, you must unlearn specific patterns from "Game Object" engines (like Unity or classic OOP).

### The "Brain in a Jar" Philosophy

Think of your game logic (Rust) as a brain floating in a jar.

* It cannot see the screen.
* It cannot hear the speakers.
* It cannot touch the DOM.

It only receives **Pulse Signals** (Inputs/Events) and produces **Snapshots** (Render Buffer).

* **Bad Pattern:** `player.jump()` triggers `audio.play("jump.mp3")` immediately.
* **Good Pattern:** `player.jump()` sets `events.push(SoundEvent::Play("jump"))`. The Host (JS) reads this event later and plays the sound.

### The "Loop" over The "Object"

In ZapEngine, **Data** is king. **Behavior** is secondary.

* Do not create classes with methods (`class Hero { update() { ... } }`).
* Create pure data structs (`struct Hero { pos: Vec2 }`) and systems that operate on them (`fn update_heroes(heroes: &mut [Hero])`).

---

## 2. The Core Rust API

The API is designed to be **Explicit** and **Testable**. We avoid global state (statics) to ensure we can run multiple game instances or run tests in parallel.

### The Contract: `impl Game`

Every game must implement this trait. It is the entry point.

```rust
pub trait Game {
    /// Initialize your world. 
    /// The 'ctx' provides access to Assets and Physics configuration.
    fn init(&mut self, ctx: &mut EngineContext);

    /// The heartbeat. Called 60 times per second.
    /// 'dt' is fixed. 'input' is a queue of events since the last frame.
    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue);
    
    /// Optional: Push debug lines or custom geometry not handled by auto-rendering.
    fn render(&self, ctx: &mut RenderContext) {}
}

```

### Context Objects (Dependency Injection)

We never call singletons like `Input::get()` or `Physics::step()`. Instead, the engine passes **Contexts**.

* **`EngineContext`**: Gives mutable access to the Physics World, Asset Manager, and Event Bus.
* **`RenderContext`**: Gives write-only access to the GPU buffers.

**Why?** This allows you to write unit tests where you pass a `MockContext`, letting you test game logic without booting up a WebGPU browser.

---

## 3. Entity Architecture: "Fat Structs"

For educational and mid-sized 2D games, a full Entity Component System (ECS) like Bevy is often overkill and conceptually difficult. We use the **Fat Entity** pattern.

### The Standard Entity

A single struct holds the state for *everything* in the game, from a UI button to a Physics Atom.

```rust
pub struct Entity {
    // 1. Identification
    pub id: EntityId,
    pub tag: String,       // e.g., "player", "enemy", "bullet"
    pub active: bool,

    // 2. Spatial
    pub pos: Vec2,
    pub rot: f32,
    pub scale: Vec2,

    // 3. Components (Optional)
    // If 'None', the system ignores it.
    pub sprite: Option<SpriteComponent>,
    pub body: Option<RigidBodyHandle>,   // Physics
    pub collider: Option<ColliderHandle>,
    pub emitter: Option<ParticleEmitter>, // VFX
    pub mesh: Option<MeshComponent>,      // 3D/SDF
}

```

### The System Pattern

Game logic is implemented as functions that iterate over entities.

```rust
// Clean, functional system design
fn system_movement(entities: &mut [Entity]) {
    for e in entities.iter_mut() {
        // If it has a body, sync physics position to visual position
        if let Some(handle) = e.body {
             // Logic handled by engine, but accessible here
        }
    }
}

```

---

## 4. The JavaScript/React Bridge

For the Front-End developer, the engine should feel like a standard React library.

### The Hook: `useZapEngine`

```typescript
// App.tsx
const { 
  canvasRef, 
  sendInput, 
  gameState 
} = useZapEngine({
  wasm: "chemistry_lab.wasm",
  assets: manifest
});

// Handling UI interactions
<button onPointerDown={() => sendInput({ type: "SPAWN_ATOM" })}>
  Add Atom
</button>

```

### State Synchronization

The engine supports a **One-Way Data Flow**.

1. **React** sends `InputEvent` → **Rust**.
2. **Rust** processes logic.
3. **Rust** sends `GameEvent` (Score update, Game Over) → **React**.

*Avoid polling.* The engine pushes events to React only when UI needs to update.

---

## 5. Coding Standards & Best Practices

### For the Game Developer (User)

1. **Functional Core, Imperative Shell**: Keep your math and logic pure. Only mutate state in the explicit `update` block.
2. **No `unwrap()` in Update**: If the game crashes, the loop dies. Use `if let` or `match`. If an asset is missing, log a warning and render a pink placeholder, but **do not crash**.
3. **Data-Driven Design**: Don't hardcode "damage = 10". Put it in a `balance.json` or constants file. The engine supports hot-reloading JSON configs.
4. **Physics vs. Transform**: If an entity has a Physics Body, **never** manually set `entity.pos`. Apply forces or velocity instead. The Physics engine owns the position.

### For the Engine Maintainer (You)

1. **Zero-Cost Abstractions**: Use Rust generics and inlining. The `Entity` struct should lay out in memory effectively.
2. **Safety First**:
* WASM/JS interop involves raw pointers (`unsafe`).
* **Rule:** Every `unsafe` block must have a `// SAFETY: ...` comment explaining why it is safe.


3. **WGPU State Management**:
* Handle `DeviceLost` errors gracefully.
* Do not crash if the user's browser doesn't support HDR. Fallback silently to sRGB.



---

## 6. Testing Strategy

One of the biggest benefits of this architecture is testability.

### Unit Tests (Rust)

Since logic is decoupled from rendering, you can test gameplay physics in CI/CD pipelines.

```rust
#[test]
fn test_gravity() {
    let mut game = MyGame::default();
    let mut ctx = MockContext::new(); // No GPU, just logic
    
    game.spawn_apple(0.0, 10.0); // Spawn at Y=10
    
    // Simulate 1 second
    for _ in 0..60 {
        game.update(&mut ctx, &InputQueue::empty());
    }
    
    assert!(game.get_apple().pos.y < 10.0); // It fell!
}

```

### Integration Tests (Browser)

We use `playwright` to load the game and check if the canvas renders (via pixel snapshotting) and if the frame counter advances.

---

## 7. Directory Structure (Clean Architecture)

```text
src/
├── api/             # The public traits (Game, EngineContext)
├── core/            # The main loop, time management
├── components/      # The component structs (Sprite, Body, Emitter)
├── systems/         # The logic (PhysicsStep, ParticleUpdate)
└── bridge/          # The WASM/JS boundary (keep this isolated!)

```
