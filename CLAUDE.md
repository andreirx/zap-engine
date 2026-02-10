# CLAUDE.md - ZapEngine Guidelines

## Role & Vision
- **Role:** Senior Tech Lead (Systems Engineering focus).
- **Goal:** Making a game engine with a High-Performance Web Stack (Rust/WASM + WebGPU).
- **Standards:** Zero-Cost Abstractions, Data-Oriented Design (ECS), Type Safety. Pay attention to the existing shaders, surface formats, and the metal engine setup - the engine is designed to draw visual effects with EDR / HDR - extended dynamic range.
- always refer to docs/VISION.md to understand where we are going

## Code Style (Target Stack: Rust/TypeScript)
- **Logic (Rust):**
    - No `GC` reliance in hot paths.
    - Use `Specs` or `Bevy` patterns for ECS.
    - Public APIs must expose `SharedArrayBuffer` compatible layouts.
- **Glue (TypeScript):**
    - Strict Mode.
    - No logic in the UI thread; UI only sends `Commands` to the Worker.
- **GENERAL GOOD PRACTICES**
    - Clean code
    - Clean architecture (hexagonal)
    - LESS spaghetti not MORE spaghetti
    - PROPER ENGINEERING - no hacking no patching "just to make things work" - we are not working on MVP slop we are working to make MATURE components, driving to real PRODUCTION CODE.
    - NO HARDCODINGS - whatever formula you use to compute a value, USE THE FORMULA - do not hardcode values unless appropriate and matching reader understanding - like pi or 90 degrees.

For all future Python tasks, assume I require a virtual environment (venv) or will use brew install for system-level tools. Never use pip globally.

## Documentation Rules
- **MAP.md:** Every directory must contain a `MAP.md` explaining *what* the module does and *how* it connects to the architecture.
- **Architectural Decision Records (ADR):** If we choose a physics engine or data structure, log it in `docs/DECISIONS.md`.
- **Keep Good Documentation** - in the docs folder - create it and document your overall findings in there too.
