# ZapEngine Game Template

Minimal starter template for new ZapEngine games. Renders a single spinning sprite.

## Quick Start

1. **Copy this directory** and rename it:
   ```bash
   cp -r examples/zap-engine-template examples/my-game
   ```

2. **Update `Cargo.toml`**: Change the package `name` to your game name.

3. **Update `src/lib.rs`**: Replace `HelloGame` with your game struct name.

4. **Add to workspace** in the root `Cargo.toml`:
   ```toml
   members = [
       # ...existing members...
       "examples/my-game",
   ]
   ```

5. **Build WASM**:
   ```bash
   wasm-pack build examples/my-game --target web --out-dir pkg
   ```

6. **Run dev server** (from the repo root):
   ```bash
   npx vite
   ```
   Open the template's `index.html` or update the root `index.html` entry to point to your game.

## Structure

| File | Purpose |
|------|---------|
| `Cargo.toml` | Rust crate config |
| `src/lib.rs` | wasm-bindgen exports (required boilerplate) |
| `src/game.rs` | Your game logic — implements the `Game` trait |
| `main.ts` | TypeScript entry — worker, renderer, input |
| `index.html` | HTML shell |
| `public/assets/` | Asset manifest + images |

## Game Trait

Your game implements `zap_engine::Game`:

```rust
impl Game for MyGame {
    fn config(&self) -> GameConfig { /* world size, physics, etc. */ }
    fn init(&mut self, ctx: &mut EngineContext) { /* spawn entities */ }
    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) { /* game logic */ }
}
```

See `examples/basic-demo/` for a more complete example with physics, particles, and collision handling.
