# Solar System — Architecture

Interactive solar system simulation with accurate Keplerian orbits.

## Rendering Pipeline

- **Sun**: SDF sphere (emissive=3.5 for HDR glow) + vector fill_circles for corona halo + effects arcs for solar flares + particles for solar wind
- **Planets**: SDF spheres with unique colors and sizes
- **Moons**: Small SDF spheres in circular orbits around parent planets
- **Asteroids**: 50 tiny SDF spheres with Keplerian orbits in the 2.2–3.2 AU belt
- **Orbit paths**: Vector stroke_polygon per planet (96-point sampled ellipses)
- **Saturn rings**: 3 concentric stroke_polygon ellipses (y-axis compressed for tilt)
- **Selection ring**: Vector stroke_circle around selected body

Draw order: vectors (corona, orbits, rings) → SDF (bodies) → effects (flares, particles)

## Coordinate System

- World: 1600×900 pixels, Sun at center (800, 450)
- Distance scaling: `screen_px = sqrt(au) * 68.0` (preserves inner/outer planet visibility)
- Planet sizes exaggerated (Sun 28px, Jupiter 14px, Earth 5px, Mercury 3px)

## Orbital Mechanics

- J2000 epoch Keplerian elements from JPL/Standish (1992)
- Newton-Raphson Kepler solver (15 iterations, 1e-12 tolerance)
- All math in f64, converted to f32 only for screen coordinates
- Moons: simplified circular orbits relative to parent planet
- Asteroids: randomized Keplerian elements (deterministic hash, no rand crate)

## Module Structure

| File | Purpose |
|------|---------|
| `orbit.rs` | Kepler equation solver, heliocentric position, Julian date conversion |
| `bodies.rs` | Planet/moon/asteroid data — J2000 elements, visual properties |
| `game.rs` | Game trait implementation, update loop, rendering, input handling |
| `lib.rs` | WASM exports (thread_local GameRunner pattern) |
| `App.tsx` | React UI — time slider, speed controls, planet legend, info card |

## React ↔ Rust Protocol

**Custom events (React → Rust):**
- kind=1: Set time (a=days_from_j2000)
- kind=2: Set speed (a=days_per_second)
- kind=3: Toggle pause
- kind=4: Select planet (a=index, -1=deselect)

**Game events (Rust → React):**
- kind=1: Time info (a=days, b=speed, c=paused)
- kind=2: Date info (a=year, b=month, c=day)
- kind=3: Selection (a=planet_index/-1, b=distance_au)
