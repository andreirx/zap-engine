# CLAUDE.md - ZapEngine Guidelines

## CRITICAL: Build Process

**ALWAYS use `wasm-pack build` or `scripts/build-all.sh`. NEVER use `cargo build` directly for WASM.**

```bash
# ✅ CORRECT — builds WASM + JS bindings together
wasm-pack build examples/solar-system --target web --out-dir pkg

# ✅ CORRECT — builds everything
bash scripts/build-all.sh

# ✅ SAFE — type checking only, no artifacts produced
cargo check --target wasm32-unknown-unknown

# ❌ WRONG — compiles WASM but does NOT regenerate JS bindings
cargo build --target wasm32-unknown-unknown
```

**Why `cargo build` alone breaks things:**
- `wasm-pack build` internally runs cargo + wasm-bindgen together
- Direct `cargo build` only compiles WASM, skips wasm-bindgen
- Result: WASM expects `__wbg_log_ABC123`, JS exports `__wbg_log_XYZ789`
- Browser error: `LinkError: function import requires a callable`

**See `docs/BUILD.md` for the complete build pipeline.**

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

# System Intent (WHY)
This repository contains a high-reliability, safety-critical product. The objective is rock-solid execution, not a Minimum Viable Product. Structural decisions must prioritize long-term maintainability, hardware-independence, and off-target testability. 

# Clean Architecture Directives (UNIVERSAL RULES)
1. **The Dependency Rule:** Source code dependencies must point strictly inward toward `core/`. Elements in `core/` must never import or reference entities from `adapters/` or `infrastructure/`.
2. **Boundary Enforcement:** Data crossing architectural boundaries must utilize simple Data Transfer Objects (DTOs). Do not pass framework-specific objects, hardware structs, or database rows across boundaries.
3. **Volatility Isolation:** Hardware, databases, and frameworks are volatile external details. Isolate them behind strict abstraction layers (e.g., HAL, OSAL, Gateways).
4. **Architectural Decisions:** When encountering an architectural fork, halt and ask for clarification. Do not unilaterally select an architecture pattern. Provide evidence and explain the underlying mechanics of available options to facilitate a decision.

# Progressive Disclosure Context
Do not assume domain specifics. Read the relevant files din docs before modifying their associated domains (and update them when the user input justifies it)
* architecture decisions: Historical context and existing structural boundaries.
* hardware abstractions: Protocols for the HAL and off-target simulation requirements.
* database schema: Persistence layer rules and Gateway interface implementations.
* testing strategy: Rules for the Test API and decoupled verification.
