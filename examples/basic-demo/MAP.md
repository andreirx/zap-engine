# basic-demo

Minimal example showcasing ZapEngine features:

- Implements `Game` trait with init/update/render
- Spawns colored sprites that bounce within world bounds
- Responds to pointer input (click to spawn new sprites)
- Demonstrates: entity creation, sprite rendering, input handling, effects

## Structure

| File | Purpose |
|---|---|
| `src/lib.rs` | `BasicDemo` game struct implementing `Game` trait |
| `index.html` | HTML entry point |
| `main.ts` | TypeScript bootstrap: init worker, connect renderer |
| `public/assets/` | Test atlas PNG + `assets.json` manifest |
