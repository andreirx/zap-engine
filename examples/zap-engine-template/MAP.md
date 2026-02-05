# zap-engine-template

Minimal starter template for new ZapEngine games.

## Purpose
Provides the boilerplate needed to start a new game: wasm-bindgen exports, a minimal `Game` implementation, TypeScript entry point, and HTML shell. Copy this directory and rename to start a new project.

## Structure
- `Cargo.toml` — Rust crate config with zap-engine + zap-web dependencies
- `src/lib.rs` — wasm-bindgen glue (thread_local GameRunner + exported functions)
- `src/game.rs` — Minimal `HelloGame` implementing the `Game` trait
- `main.ts` — TypeScript entry: worker init, renderer, render loop, input forwarding
- `index.html` — HTML shell with fullscreen canvas
- `public/assets/assets.json` — Minimal asset manifest (reuses demo_tiles atlas)

## Architecture Connection
This is the **copy-and-start** entry point for the engine. It follows the same pattern as `basic-demo` but with the absolute minimum code needed to render a spinning sprite. Games implement the `Game` trait in Rust, which the `GameRunner` in zap-web drives via `init()` / `tick()`. The TypeScript side manages the Worker, SharedArrayBuffer protocol, and WebGPU/Canvas2D renderer.
