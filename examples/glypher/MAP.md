# glypher/

A new game built on ZapEngine. Currently scaffolded with a minimal `Game` trait implementation.

## Structure

| File | Purpose |
|---|---|
| `Cargo.toml` | Rust crate config — depends on `zap-engine` + `zap-web` with physics/vectors features |
| `src/lib.rs` | WASM exports wrapping `GameRunner<Glypher>` |
| `src/game.rs` | `Game` trait implementation (800x600, 60fps) |
| `App.tsx` | React component using `useZapEngine` from `@zap/web/react` |
| `main.tsx` | Entry point — renders `<App />` |
| `index.html` | Vite HTML entry |
| `public/assets/assets.json` | Asset manifest (currently empty) |

## Build

```bash
wasm-pack build examples/glypher --target web --out-dir pkg
```
