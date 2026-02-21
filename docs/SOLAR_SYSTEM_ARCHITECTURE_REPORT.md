# Solar System Architecture Report

## Executive Summary

The solar-system example has grown organically into a **Big Ball of Mud** — a monolithic 700+ line file where coordinate transformations, input handling, orbital mechanics, visual effects, and UI concerns are entangled. The recent zoom-to-point debugging exposed a fundamental architectural failure: **coordinate spaces are implicit rather than explicit first-class citizens**.

This report applies Clean Architecture principles to deconstruct the problem and propose a major refactor.

---

## Phase 1: Problem Deconstruction

### 1.1 Actor Analysis

The system serves multiple actors whose requirements change independently:

| Actor | Concerns | Change Frequency |
|-------|----------|------------------|
| **Astronomer** | Orbital accuracy, Keplerian elements, Julian dates | Rare (physics is stable) |
| **UI Designer** | Planet selection, time controls, visual feedback | Frequent |
| **Graphics Engineer** | SDF rendering, effects, camera behavior | Moderate |
| **Player** | Zoom, pan, click interactions | Stable interface, volatile implementation |

**Violation:** Currently, a change to zoom behavior requires modifying the same file as orbital mechanics. These actors should be isolated.

### 1.2 Core Policy vs Mechanisms

**Core Policy (Entities — stable, hardware-independent):**
- Keplerian orbital mechanics (`OrbitalElements`, `heliocentric_position`)
- Julian date calculations (`days_to_centuries`, `days_to_calendar`)
- Celestial body data (radii, colors, orbital parameters)
- Coordinate space definitions (AU space, base space, screen space)

**Mechanisms (volatile, replaceable):**
- Camera transform (could be 2D orthographic, 3D perspective, VR)
- Input handling (mouse, touch, gamepad)
- Visual rendering (SDF spheres, sprites, ASCII art)
- UI feedback (selection rings, info panels)

**Violation:** `base_to_screen()`, `au_to_screen()`, and camera state are deeply intertwined with game state instead of being injected abstractions.

### 1.3 Coordinate Space Volatility Profile

**CRITICAL INSIGHT:** Coordinate spaces are FIRST-CLASS CITIZENS, not implementation details.

| Space | Definition | Stability |
|-------|-----------|-----------|
| **AU Space** | Heliocentric ecliptic coordinates (astronomical units) | Stable (defined by physics) |
| **Base Space** | Pixel coordinates at zoom=1, camera=(0,0) | Stable (design choice) |
| **World Space** | Screen-aligned pixels after camera transform | Volatile (depends on camera) |
| **Screen Space** | CSS pixels in browser viewport | Volatile (depends on viewport) |
| **NDC Space** | Normalized device coordinates [-1,1] | Stable (GPU standard) |

**Current Violation:** These spaces are implicit. The code has `base_to_screen()` but no explicit `AuSpace`, `BaseSpace`, or `WorldSpace` types. Transformations are ad-hoc f32 tuples scattered across methods.

### 1.4 Current Dependency Violations

```
┌─────────────────────────────────────────────────────────────────┐
│                        game.rs (MONOLITH)                        │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐  │
│  │ OrbitalMech │◄─┤ CameraState  │◄─┤ InputHandling          │  │
│  │ (stable)    │  │ (volatile)   │  │ (volatile)             │  │
│  └─────────────┘  └──────────────┘  └────────────────────────┘  │
│         ▲                ▲                      ▲                │
│         │                │                      │                │
│         └────────────────┴──────────────────────┘                │
│                    CIRCULAR DEPENDENCIES                         │
└─────────────────────────────────────────────────────────────────┘
```

The `SolarSystem` struct owns:
- `planet_elements: [OrbitalElements; 9]` (core policy)
- `cam_x, cam_y, zoom` (camera mechanism)
- `visible_w, visible_h` (viewport mechanism)
- `dragging, drag_start, drag_moved` (input state)
- `selected, flare_timer` (UI state)

**All concerns mixed in one struct = SRP violation.**

---

## Phase 2: Proposed Architecture

### 2.1 Layered Structure (Dependency Rule)

```
┌─────────────────────────────────────────────────────────────────┐
│                         MAIN (wiring)                            │
│  Creates concrete instances, injects dependencies, starts loop  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    USE CASES / INTERACTORS                       │
│  OrreryInteractor: orchestrates simulation, input→commands       │
└─────────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────┐
│    ENTITIES     │ │   GATEWAYS      │ │      PRESENTERS         │
│ OrbitalMechanics│ │ CameraPort      │ │ SolarSystemPresenter    │
│ CelestialBody   │ │ InputPort       │ │ (transforms to render)  │
│ CoordinateSpaces│ │ ViewportPort    │ │                         │
└─────────────────┘ └─────────────────┘ └─────────────────────────┘
          ▲                   ▲                   ▲
          │                   │                   │
          │         DEPENDS INWARD ONLY           │
          │                   │                   │
┌─────────────────────────────────────────────────────────────────┐
│                    ADAPTERS / INFRASTRUCTURE                     │
│  OrbitCamera2D, ZapInputAdapter, WebViewport, SDFRenderer        │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Coordinate Spaces as First-Class Types

```rust
// coordinates.rs — ENTITIES layer (stable, no dependencies)

/// Marker trait for coordinate space type safety.
pub trait CoordinateSpace: Copy {}

/// Astronomical Units — heliocentric ecliptic plane.
#[derive(Debug, Clone, Copy, Default)]
pub struct AuSpace;
impl CoordinateSpace for AuSpace {}

/// Base pixels — zoom=1, camera at origin. 1 AU ≈ some constant pixels.
#[derive(Debug, Clone, Copy, Default)]
pub struct BaseSpace;
impl CoordinateSpace for BaseSpace {}

/// World pixels — after camera transform (pan + zoom).
#[derive(Debug, Clone, Copy, Default)]
pub struct WorldSpace;
impl CoordinateSpace for WorldSpace {}

/// Screen pixels — CSS coordinates in viewport.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScreenSpace;
impl CoordinateSpace for ScreenSpace {}

/// Type-safe 2D point in a specific coordinate space.
#[derive(Debug, Clone, Copy, Default)]
pub struct Point<S: CoordinateSpace> {
    pub x: f64,
    pub y: f64,
    _space: std::marker::PhantomData<S>,
}

impl<S: CoordinateSpace> Point<S> {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y, _space: std::marker::PhantomData }
    }
}

/// Transform from one coordinate space to another.
/// Implemented as separate structs to enforce explicit conversion.
pub trait Transform<From: CoordinateSpace, To: CoordinateSpace> {
    fn transform(&self, point: Point<From>) -> Point<To>;
    fn inverse(&self, point: Point<To>) -> Point<From>;
}
```

**Benefit:** You literally CANNOT pass AU coordinates where screen coordinates are expected. The compiler enforces coordinate space boundaries.

### 2.3 Camera as Injectable Port

```rust
// ports/camera.rs — GATEWAY layer (interface only, no impl)

/// Camera abstraction — core logic doesn't know if it's 2D, 3D, or VR.
pub trait CameraPort {
    type WorldCoord: CoordinateSpace;
    type ScreenCoord: CoordinateSpace;

    /// Project world point to screen.
    fn project(&self, world: Point<Self::WorldCoord>) -> Point<Self::ScreenCoord>;

    /// Unproject screen point to world (for hit testing).
    fn unproject(&self, screen: Point<Self::ScreenCoord>) -> Point<Self::WorldCoord>;

    /// Visible bounds in world space (for culling).
    fn visible_bounds(&self) -> Rect<Self::WorldCoord>;

    /// Zoom factor (for scaling visuals).
    fn zoom(&self) -> f64;
}

/// Camera commands — core logic emits these, adapter handles them.
pub enum CameraCommand {
    Pan { delta_x: f64, delta_y: f64 },
    ZoomToward { screen_point: Point<ScreenSpace>, factor: f64 },
    Reset,
}
```

### 2.4 Orbital Mechanics as Pure Entity

```rust
// entities/orbital.rs — ENTITIES layer (pure math, no I/O)

/// Keplerian orbital elements at J2000 epoch with secular rates.
#[derive(Debug, Clone, Copy)]
pub struct OrbitalElements {
    pub a0: f64,      // Semi-major axis (AU)
    pub e0: f64,      // Eccentricity
    pub i0: f64,      // Inclination (deg)
    pub l0: f64,      // Mean longitude (deg)
    pub w0: f64,      // Longitude of perihelion (deg)
    pub o0: f64,      // Longitude of ascending node (deg)
    // Secular rates per century...
}

impl OrbitalElements {
    /// Compute heliocentric position at given Julian centuries from J2000.
    /// Returns (x_au, y_au) in ecliptic plane.
    ///
    /// This is PURE MATH — no rendering, no camera, no side effects.
    pub fn position_at(&self, t_centuries: f64) -> Point<AuSpace> {
        // Kepler's equation solving...
        Point::new(x_au, y_au)
    }
}

/// Celestial body definition — immutable data.
pub struct CelestialBody {
    pub name: &'static str,
    pub elements: OrbitalElements,
    pub radius_km: f64,
    pub color: (f32, f32, f32),
    pub is_dwarf_planet: bool,
}

/// The solar system — static astronomical data.
pub struct SolarSystemData {
    pub sun: SunData,
    pub planets: [CelestialBody; 8],
    pub dwarf_planets: [CelestialBody; 1], // Pluto
    pub moons: Vec<MoonData>,
    pub asteroid_belt: AsteroidBeltParams,
}
```

### 2.5 Use Case / Interactor

```rust
// use_cases/orrery.rs — USE CASE layer (orchestration)

/// The orrery simulation — coordinates entities with gateways.
pub struct OrreryInteractor<C: CameraPort, V: ViewportPort> {
    data: SolarSystemData,
    time: SimulationTime,
    camera: C,
    viewport: V,
    selection: Option<usize>,

    // Transform chain (injected, not owned)
    au_to_base: AuToBaseTransform,
}

impl<C: CameraPort, V: ViewportPort> OrreryInteractor<C, V> {
    /// Advance simulation time.
    pub fn tick(&mut self, dt: f32) {
        self.time.advance(dt);
    }

    /// Get all body positions in world space for rendering.
    pub fn body_positions(&self) -> impl Iterator<Item = BodyRenderData> {
        let t = self.time.as_centuries();

        self.data.planets.iter().map(move |body| {
            let au_pos = body.elements.position_at(t);
            let base_pos = self.au_to_base.transform(au_pos);
            let world_pos = self.camera.project_from_base(base_pos);

            BodyRenderData {
                position: world_pos,
                radius: body.radius_km * self.camera.zoom() * RADIUS_SCALE,
                color: body.color,
            }
        })
    }

    /// Handle camera command (from input adapter).
    pub fn handle_camera(&mut self, cmd: CameraCommand) {
        match cmd {
            CameraCommand::ZoomToward { screen_point, factor } => {
                // The camera adapter implements the actual math
                self.camera.zoom_toward(screen_point, factor);
            }
            // ...
        }
    }
}
```

---

## Phase 3: Component Structure

### 3.1 Directory Layout

```
examples/solar-system/src/
├── main.rs                    # MAIN — wiring only
├── lib.rs                     # WASM exports (thin wrapper)
│
├── entities/                  # ENTITIES — stable core
│   ├── mod.rs
│   ├── coordinates.rs         # Point<Space>, Transform trait
│   ├── orbital.rs            # OrbitalElements, Kepler solver
│   ├── bodies.rs             # CelestialBody, SolarSystemData
│   └── time.rs               # JulianDate, SimulationTime
│
├── use_cases/                 # USE CASES — application logic
│   ├── mod.rs
│   ├── orrery.rs             # OrreryInteractor
│   └── commands.rs           # CameraCommand, SelectionCommand
│
├── ports/                     # PORTS — interfaces (traits)
│   ├── mod.rs
│   ├── camera.rs             # CameraPort trait
│   ├── input.rs              # InputPort trait
│   └── viewport.rs           # ViewportPort trait
│
├── adapters/                  # ADAPTERS — implementations
│   ├── mod.rs
│   ├── orbit_camera.rs       # OrbitCamera2D implements CameraPort
│   ├── zap_input.rs          # ZapInputAdapter implements InputPort
│   └── zap_viewport.rs       # Handles resize events
│
└── presenter/                 # PRESENTER — render data assembly
    ├── mod.rs
    └── orrery_presenter.rs   # Transforms OrreryInteractor → RenderData
```

### 3.2 Dependency Graph (DAG, No Cycles)

```
main.rs
   │
   ▼
adapters/*  ──────────────┐
   │                      │
   ▼                      ▼
presenter/* ◄──────── use_cases/*
                          │
                          ▼
                      ports/* (traits only)
                          │
                          ▼
                      entities/* (pure, no deps)
```

### 3.3 Transform Chain (Explicit)

```rust
// The full transform pipeline, each step is explicit:

Point<AuSpace>           // From orbital mechanics
    │
    ▼ AuToBaseTransform  // 1 AU = 150 pixels (configurable)
    │
Point<BaseSpace>         // Stable reference frame
    │
    ▼ CameraTransform    // Pan + zoom
    │
Point<WorldSpace>        // What the renderer sees
    │
    ▼ ProjectionMatrix   // GPU handles this
    │
Point<NDCSpace>          // [-1, 1] clip space
```

---

## Phase 4: Testing Strategy

### 4.1 Off-Target Testability

With this architecture, **100% of core logic is testable without WASM, without a canvas, without a browser:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Mock camera that records commands
    struct MockCamera {
        zoom_calls: Vec<(Point<ScreenSpace>, f64)>,
        current_zoom: f64,
    }

    impl CameraPort for MockCamera {
        fn zoom_toward(&mut self, pt: Point<ScreenSpace>, factor: f64) {
            self.zoom_calls.push((pt, factor));
            self.current_zoom *= factor;
        }
        // ...
    }

    #[test]
    fn zoom_toward_point_keeps_point_stationary() {
        let mut camera = OrbitCamera2D::new(1600.0, 900.0);
        let target = Point::<ScreenSpace>::new(300.0, 200.0);

        // Convert to world before zoom
        let world_before = camera.unproject(target);

        // Zoom in
        camera.zoom_toward(target, 1.5);

        // Convert same screen point to world after zoom
        let world_after = camera.unproject(target);

        // The world point should be the same!
        assert!((world_before.x - world_after.x).abs() < 0.001);
        assert!((world_before.y - world_after.y).abs() < 0.001);
    }

    #[test]
    fn orbital_position_matches_nasa_horizons() {
        let earth = OrbitalElements::earth();
        let t = 0.0; // J2000 epoch
        let pos = earth.position_at(t);

        // Compare against NASA Horizons ephemeris
        assert!((pos.x - 0.9833).abs() < 0.001); // ~1 AU
    }
}
```

### 4.2 Test API Location

Tests interact with `OrreryInteractor` directly — NOT through the WASM boundary, NOT through the renderer. The Test API is the use case layer itself.

---

## Phase 5: Migration Plan

### Step 1: Extract Coordinate Types (Low Risk)
- Create `entities/coordinates.rs` with `Point<S>` and `Transform` trait
- Replace bare `(f32, f32)` tuples with typed points
- Compiler errors guide the migration

### Step 2: Extract Orbital Mechanics (Low Risk)
- Move `OrbitalElements` and `heliocentric_position` to `entities/orbital.rs`
- Move `bodies.rs` constants to `entities/bodies.rs`
- Pure refactor — no behavior change

### Step 3: Define Camera Port (Medium Risk)
- Create `ports/camera.rs` with `CameraPort` trait
- Implement `OrbitCamera2D` in `adapters/`
- Replace inline camera state with injected camera

### Step 4: Create Interactor (Medium Risk)
- Move simulation logic to `OrreryInteractor`
- `SolarSystem` struct becomes thin adapter
- Game trait impl delegates to interactor

### Step 5: Extract Presenter (Low Risk)
- Move render data assembly to `presenter/`
- Interactor returns domain objects, presenter transforms to render instances

---

## Conclusion

The current solar-system code violates:
- **SRP:** One struct handles orbital mechanics, camera, input, selection, and rendering
- **OCP:** Adding a new camera type requires modifying `SolarSystem`
- **DIP:** Core logic depends on concrete camera implementation
- **CCP:** Camera changes force recompilation of orbital mechanics
- **Dependency Rule:** No clear boundary between stable and volatile components

The proposed architecture:
- Makes coordinate spaces **explicit types** (compile-time enforcement)
- Separates **entities** (stable math) from **mechanisms** (volatile camera/input)
- Enables **off-target testing** without WASM or browser
- Supports **future extensions** (3D view, VR mode, different renderers)

The coordinate space confusion that caused the zoom-to-point bug would have been **impossible** with typed coordinates — the compiler would have caught the mismatch.
