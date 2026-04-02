# effects-showcase

Demonstrates the three Phase 13-14 engine capabilities added to zap-engine:

1. **Per-sprite blend mode** — additive-blend orbs orbit the center with HDR glow
2. **Alpha-blended particles** — continuous smoke emitter (left) on VFX layer, click-spawned smoke clouds
3. **Visibility mask** — 40x30 grid with linear interpolation, circular reveal follows pointer, UI layer excluded

Also uses: dynamic lighting (ambient + viewpoint follow light + explosion decay lights), additive particle sparks (right), programmatic atlas generation (no external PNG assets).

## Files

- `src/lib.rs` — `export_game!` macro wiring
- `src/game.rs` — `EffectsShowcase` game implementation (Game trait)
- `main.ts` — Standalone TypeScript loader with programmatic atlas and full protocol plumbing
- `index.html` — Entry point with usage instructions overlay

## Architecture connection

This is a standalone example crate, not a library. It depends on `zap-engine` (Rust game logic) and `zap-web` (WASM bridge + worker protocol). The main.ts loader mirrors the pattern used by `useZapEngine` but without React, using `readFrameState` directly.
