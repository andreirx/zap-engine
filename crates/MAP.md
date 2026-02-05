# crates/

Rust workspace members. Each crate has a distinct responsibility.

| Crate | Type | Purpose |
|---|---|---|
| `zap-engine` | rlib | Core engine library — pure Rust, no WASM dependencies. Defines the `Game` trait, entity system, render buffer protocol, and all engine systems. |
| `zap-web` | cdylib + rlib | WASM bridge — provides `GameRunner<G: Game>` that wires up the game loop and exposes pointer accessors for SharedArrayBuffer reads from TypeScript. |
