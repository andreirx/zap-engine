# zap-web

WASM bridge crate. Provides `GameRunner<G: Game>` — the glue between a Rust game and the browser.

## Architecture

`GameRunner` is generic over `G: Game`. It owns:
- The game instance
- `EngineContext` (scene, sounds, events)
- `InputQueue`
- `RenderBuffer`
- `FixedTimestep`

Because `wasm-bindgen` cannot export generic structs, each concrete game (e.g., `basic-demo`)
creates a `thread_local!` GameRunner and exports free functions that delegate to it.
`GameRunner::game()` and `GameRunner::game_mut()` intentionally expose the concrete game back to those free functions for game-specific JSON boundaries such as example-local level save/load.

## Key Files

| File | Purpose |
|---|---|
| `lib.rs` | Crate root, re-exports `GameRunner` |
| `runner.rs` | `GameRunner<G>` implementation: init, tick, pointer accessors |
