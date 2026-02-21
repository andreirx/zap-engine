/// Solar System — interactive orrery with Keplerian orbits.
///
/// Pure SDF + vectors + effects — no sprites, no physics.
/// Camera system: drag-to-pan, scroll-to-zoom, viewport-adaptive centering.

use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::input::queue::{InputEvent, InputQueue};
use zap_engine::components::mesh::{MeshComponent, SDFColor, SDFShape};
use zap_engine::systems::effects::SegmentColor;
use glam::Vec2;

use crate::bodies;
use crate::orbit::{self, OrbitalElements};

// ── World layout ─────────────────────────────────────────────────────

const WORLD_W: f32 = 1600.0;
const WORLD_H: f32 = 900.0;

/// Distance scaling: screen_px = sqrt(au) * SCALE_FACTOR.
/// Keeps inner planets visible while fitting Pluto on screen.
const SCALE_FACTOR: f64 = 90.0;

/// Number of sample points for orbit path drawing.
const ORBIT_SAMPLES: usize = 96;
/// Orbit line width in pixels.
const ORBIT_LINE_WIDTH: f32 = 0.5;

// ── Custom event kinds from React ────────────────────────────────────

const CUSTOM_SET_DAYS: u32 = 1;
const CUSTOM_SET_SPEED: u32 = 2;
const CUSTOM_TOGGLE_PAUSE: u32 = 3;
const CUSTOM_SELECT: u32 = 4;
const CUSTOM_ZOOM: u32 = 5;
const CUSTOM_RESET_VIEW: u32 = 6;
/// Viewport resize (sent by worker as kind=99).
const CUSTOM_RESIZE: u32 = 99;

// ── Game event kinds to React ────────────────────────────────────────

const EVENT_TIME_INFO: f32 = 1.0;
const EVENT_DATE_INFO: f32 = 2.0;
const EVENT_SELECTION: f32 = 3.0;

// ── Orbit path colors (muted, semi-transparent) ─────────────────────

const ORBIT_COLORS: [(f32, f32, f32); 9] = [
    (0.6, 0.5, 0.4),   // Mercury — brownish
    (0.8, 0.7, 0.4),   // Venus — golden
    (0.3, 0.5, 0.8),   // Earth — blue
    (0.7, 0.3, 0.2),   // Mars — reddish
    (0.7, 0.6, 0.4),   // Jupiter — tan
    (0.7, 0.65, 0.4),  // Saturn — golden
    (0.4, 0.6, 0.7),   // Uranus — teal
    (0.3, 0.4, 0.7),   // Neptune — blue
    (0.5, 0.45, 0.4),  // Pluto — dull brown
];
const ORBIT_ALPHA: f32 = 0.25;

// ── Saturn ring parameters ───────────────────────────────────────────

const RING_RADII: [f32; 3] = [18.0, 22.0, 26.0];
const RING_TILT: f32 = 0.4; // Y-axis compression for visual tilt
const RING_COLOR: (f32, f32, f32) = (0.8, 0.7, 0.5);
const RING_ALPHA: f32 = 0.3;
const RING_SAMPLES: usize = 48;

// ── Solar effects ────────────────────────────────────────────────────

const FLARE_INTERVAL: f32 = 0.3; // seconds between flare spawns
const CORONA_LAYERS: usize = 4;
const CORONA_BASE_RADIUS: f32 = 5.0;
const CORONA_ALPHA_START: f32 = 0.12;
const CORONA_ALPHA_STEP: f32 = 0.025;

// ── Selection ────────────────────────────────────────────────────────

const SELECT_RING_COLOR: VectorColor = VectorColor::new(1.0, 1.0, 1.0, 0.6);
const SELECT_RING_WIDTH: f32 = 1.5;
const HIT_RADIUS_EXTRA: f32 = 12.0;

// ── Camera / pan / zoom ─────────────────────────────────────────────

/// World-pixel drag distance before a click becomes a drag.
const DRAG_THRESHOLD: f32 = 5.0;
const ZOOM_MIN: f64 = 0.15;
const ZOOM_MAX: f64 = 8.0;
/// Multiplicative zoom per scroll tick.
const ZOOM_STEP: f64 = 1.01;

// ── Coordinate conversion ────────────────────────────────────────────

/// Convert AU coordinates to "base" screen space (pixels from sun, before camera).
fn au_to_base(x_au: f64, y_au: f64) -> (f64, f64) {
    let dist = (x_au * x_au + y_au * y_au).sqrt();
    if dist < 1e-10 {
        return (0.0, 0.0);
    }
    let screen_dist = dist.sqrt() * SCALE_FACTOR;
    (x_au / dist * screen_dist, y_au / dist * screen_dist)
}

/// Update the radius of a Sphere-shaped MeshComponent.
fn set_sphere_radius(mesh: &mut Option<MeshComponent>, new_radius: f32) {
    if let Some(ref mut m) = mesh {
        if let SDFShape::Sphere { ref mut radius } = m.shape {
            *radius = new_radius;
        }
    }
}

// ── Game struct ──────────────────────────────────────────────────────

pub struct SolarSystem {
    /// Days from J2000 epoch (high-precision time accumulator).
    days: f64,
    /// Simulation speed in days per real second.
    speed: f64,
    /// Paused state.
    paused: bool,
    /// Selected planet index (0–8), or None.
    selected: Option<usize>,
    /// Flare spawn timer.
    flare_timer: f32,

    // Entity IDs
    sun_id: Option<EntityId>,
    planet_ids: [Option<EntityId>; bodies::PLANET_COUNT],
    moon_ids: Vec<EntityId>,
    asteroid_ids: Vec<EntityId>,

    // Cached data
    planet_elements: [OrbitalElements; bodies::PLANET_COUNT],
    asteroid_orbits: Vec<OrbitalElements>,
    /// Base radii of asteroids (before zoom scaling).
    asteroid_base_radii: Vec<f32>,
    /// Current screen positions of planets (updated each frame).
    planet_screen_pos: [(f32, f32); bodies::PLANET_COUNT],

    // Camera state
    /// Camera offset in base coords (pixels from sun center, pre-zoom).
    cam_x: f64,
    cam_y: f64,
    /// Zoom level (1.0 = default, >1 = zoomed in).
    zoom: f64,
    /// Viewport size in world units (from resize event kind 99).
    visible_w: f32,
    visible_h: f32,

    // Drag state
    dragging: bool,
    drag_moved: bool,
    drag_start: (f32, f32),
    drag_cam_start: (f64, f64),
}

impl SolarSystem {
    pub fn new() -> Self {
        Self {
            // Start at current-ish date: ~Jan 1 2024 ≈ J2000 + 8766 days
            days: 8766.0,
            speed: 10.0, // 10 days per second — Earth visibly orbits
            paused: false,
            selected: None,
            flare_timer: 0.0,

            sun_id: None,
            planet_ids: [None; bodies::PLANET_COUNT],
            moon_ids: Vec::new(),
            asteroid_ids: Vec::new(),

            planet_elements: bodies::planet_elements(),
            asteroid_orbits: bodies::generate_asteroid_orbits(),
            asteroid_base_radii: Vec::new(),
            planet_screen_pos: [(0.0, 0.0); bodies::PLANET_COUNT],

            cam_x: 0.0,
            cam_y: 0.0,
            zoom: 1.0,
            visible_w: WORLD_W,
            visible_h: WORLD_H,

            dragging: false,
            drag_moved: false,
            drag_start: (0.0, 0.0),
            drag_cam_start: (0.0, 0.0),
        }
    }

    // ── Camera helpers ─────────────────────────────────────────────

    /// Screen center in world coordinates.
    fn screen_center(&self) -> (f32, f32) {
        (self.visible_w / 2.0, self.visible_h / 2.0)
    }

    /// Convert base coords (pixels from sun, pre-zoom) to screen/world coords.
    fn base_to_screen(&self, bx: f64, by: f64) -> (f32, f32) {
        let (cx, cy) = self.screen_center();
        (
            cx + ((bx - self.cam_x) * self.zoom) as f32,
            cy + ((by - self.cam_y) * self.zoom) as f32,
        )
    }

    /// Convert AU coordinates to screen position through camera transform.
    fn au_to_screen(&self, x_au: f64, y_au: f64) -> (f32, f32) {
        let (bx, by) = au_to_base(x_au, y_au);
        self.base_to_screen(bx, by)
    }

    /// Sun screen position (AU origin through camera).
    fn sun_screen(&self) -> (f32, f32) {
        self.base_to_screen(0.0, 0.0)
    }

    /// Zoom toward a screen point (keeps that point fixed).
    /// Uses the formula: worldX = (screenX - cx) / zoom + cam
    ///                   newCam = worldX - (screenX - cx) / newZoom
    fn zoom_toward(&mut self, screen_x: f32, screen_y: f32, new_zoom: f64) {
        let new_zoom = new_zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        let old_zoom = self.zoom;
        if (new_zoom - old_zoom).abs() < 1e-10 {
            return;
        }
        let (cx, cy) = self.screen_center();

        // Get world point under cursor
        let world_x = (screen_x as f64 - cx as f64) / old_zoom + self.cam_x;
        let world_y = (screen_y as f64 - cy as f64) / old_zoom + self.cam_y;

        // Calculate new camera position so world point stays at same screen position
        self.cam_x = world_x - (screen_x as f64 - cx as f64) / new_zoom;
        self.cam_y = world_y - (screen_y as f64 - cy as f64) / new_zoom;
        self.zoom = new_zoom;
    }

    // ── Position helpers ───────────────────────────────────────────

    /// Compute moon position relative to parent planet (simple circular orbit).
    fn moon_position(parent_pos: (f32, f32), orbit_radius: f32, period_days: f64, days: f64, zoom: f64) -> (f32, f32) {
        let angle = std::f64::consts::TAU * days / period_days;
        let r = orbit_radius * zoom as f32;
        let mx = parent_pos.0 + r * angle.cos() as f32;
        let my = parent_pos.1 + r * angle.sin() as f32;
        (mx, my)
    }

    // ── Drawing ────────────────────────────────────────────────────

    /// Draw all orbit paths as vector stroked polygons.
    fn draw_orbits(&self, vectors: &mut VectorState, t_centuries: f64) {
        for i in 0..bodies::PLANET_COUNT {
            let (r, g, b) = ORBIT_COLORS[i];
            let color = VectorColor::new(r, g, b, ORBIT_ALPHA);
            let points = self.orbit_points(&self.planet_elements[i], t_centuries);
            vectors.stroke_polygon(&points, ORBIT_LINE_WIDTH, color);
        }
    }

    /// Generate orbit path points by sampling the eccentric anomaly,
    /// transformed through the camera.
    fn orbit_points(&self, elements: &OrbitalElements, t_centuries: f64) -> Vec<Vec2> {
        let a = elements.a0;
        let e = elements.e0;
        let w = (elements.w0 + elements.w_dot * t_centuries) * std::f64::consts::PI / 180.0;

        let mut points = Vec::with_capacity(ORBIT_SAMPLES);
        for i in 0..ORBIT_SAMPLES {
            let ea = (i as f64 / ORBIT_SAMPLES as f64) * std::f64::consts::TAU;
            // Position from eccentric anomaly (in orbital frame)
            let x_orb = a * (ea.cos() - e);
            let y_orb = a * (1.0 - e * e).sqrt() * ea.sin();
            // Rotate by longitude of perihelion into ecliptic frame
            let x_au = x_orb * w.cos() - y_orb * w.sin();
            let y_au = x_orb * w.sin() + y_orb * w.cos();
            let (sx, sy) = self.au_to_screen(x_au, y_au);
            points.push(Vec2::new(sx, sy));
        }
        points
    }

    /// Draw sun corona halos (filled circles behind the sun, scaled by zoom).
    fn draw_corona(&self, vectors: &mut VectorState) {
        let (sx, sy) = self.sun_screen();
        let center = Vec2::new(sx, sy);
        let z = self.zoom as f32;
        for i in 0..CORONA_LAYERS {
            let r = (bodies::SUN_RADIUS_PX + (i as f32 + 1.0) * CORONA_BASE_RADIUS) * z;
            let alpha = CORONA_ALPHA_START - i as f32 * CORONA_ALPHA_STEP;
            if alpha > 0.0 {
                vectors.fill_circle(center, r, VectorColor::new(1.0, 0.85, 0.3, alpha));
            }
        }
    }

    /// Draw Saturn's rings as inclined ellipses (scaled by zoom).
    fn draw_saturn_rings(&self, vectors: &mut VectorState) {
        let (sx, sy) = self.planet_screen_pos[bodies::SATURN];
        let center = Vec2::new(sx, sy);
        let color = VectorColor::new(RING_COLOR.0, RING_COLOR.1, RING_COLOR.2, RING_ALPHA);
        let z = self.zoom as f32;

        for &ring_r in &RING_RADII {
            let scaled_r = ring_r * z;
            let mut points = Vec::with_capacity(RING_SAMPLES);
            for j in 0..RING_SAMPLES {
                let angle = (j as f32 / RING_SAMPLES as f32) * std::f32::consts::TAU;
                points.push(Vec2::new(
                    center.x + scaled_r * angle.cos(),
                    center.y + scaled_r * RING_TILT * angle.sin(),
                ));
            }
            vectors.stroke_polygon(&points, 1.5, color);
        }
    }

    /// Draw selection ring around selected body (radius scales with zoom).
    fn draw_selection_ring(&self, vectors: &mut VectorState) {
        if let Some(idx) = self.selected {
            let visuals = bodies::planet_visuals();
            let (sx, sy) = self.planet_screen_pos[idx];
            let radius = visuals[idx].radius_px * self.zoom as f32 + 6.0;
            vectors.stroke_circle(Vec2::new(sx, sy), radius, SELECT_RING_WIDTH, SELECT_RING_COLOR);
        }
    }

    /// Add solar flare arcs emanating from the sun (positions track camera).
    fn update_solar_flares(&mut self, effects: &mut EffectsState, dt: f32) {
        self.flare_timer += dt;
        if self.flare_timer >= FLARE_INTERVAL {
            self.flare_timer -= FLARE_INTERVAL;

            let (sun_x, sun_y) = self.sun_screen();
            let z = self.zoom as f32;
            let angle = (effects.rng.next_int(1000) as f32 / 1000.0) * std::f32::consts::TAU;
            let r = bodies::SUN_RADIUS_PX * 0.8 * z;
            let start = [
                sun_x + r * angle.cos(),
                sun_y + r * angle.sin(),
            ];
            let flare_len = (8.0 + (effects.rng.next_int(100) as f32 / 100.0) * 16.0) * z;
            let end = [
                sun_x + (r + flare_len) * angle.cos(),
                sun_y + (r + flare_len) * angle.sin(),
            ];
            let colors = [SegmentColor::Yellow, SegmentColor::Orange, SegmentColor::Red];
            let color = colors[effects.rng.next_int(3) as usize];
            effects.add_arc(start, end, 2.0, color, 3);
        }
    }

    /// Spawn solar wind particles radiating outward (scaled by zoom).
    fn spawn_solar_wind(&self, effects: &mut EffectsState) {
        let (sun_x, sun_y) = self.sun_screen();
        let z = self.zoom as f32;
        let angle = (effects.rng.next_int(10000) as f32 / 10000.0) * std::f32::consts::TAU;
        let r = (bodies::SUN_RADIUS_PX + 5.0) * z;
        let pos = [sun_x + r * angle.cos(), sun_y + r * angle.sin()];
        let speed = (3.0 + (effects.rng.next_int(100) as f32 / 100.0) * 4.0) * z;
        let sx = speed * angle.cos();
        let sy = speed * angle.sin();
        let color = SegmentColor::Yellow;
        effects.particles.push(zap_engine::Particle::new(
            pos, [sx, sy], 1.5, color, 1.5,
        ));
    }

    /// Hit-test planets, returning planet index or None.
    fn hit_test(&self, pos: Vec2) -> Option<usize> {
        let visuals = bodies::planet_visuals();
        let z = self.zoom as f32;

        let mut best: Option<(usize, f32)> = None;
        for i in 0..bodies::PLANET_COUNT {
            let (px, py) = self.planet_screen_pos[i];
            let dist = ((pos.x - px).powi(2) + (pos.y - py).powi(2)).sqrt();
            let hit_r = visuals[i].radius_px * z + HIT_RADIUS_EXTRA;
            if dist < hit_r {
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((i, dist));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// Compute distance from sun in AU for a planet at current time.
    fn planet_distance_au(&self, planet_idx: usize) -> f64 {
        let t = orbit::days_to_centuries(self.days);
        let (x, y) = orbit::heliocentric_position(&self.planet_elements[planet_idx], t);
        (x * x + y * y).sqrt()
    }
}

impl Game for SolarSystem {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: 1.0 / 60.0,
            world_width: WORLD_W,
            world_height: WORLD_H,
            max_sdf_instances: 140,
            max_events: 64,
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        let visuals = bodies::planet_visuals();
        let t = orbit::days_to_centuries(self.days);

        // ── Spawn Sun ────────────────────────────────────────────────
        let (sun_x, sun_y) = self.sun_screen();
        let sun_id = ctx.next_id();
        ctx.scene.spawn(
            Entity::new(sun_id)
                .with_tag("sun")
                .with_pos(Vec2::new(sun_x, sun_y))
                .with_mesh(
                    MeshComponent::sphere(
                        bodies::SUN_RADIUS_PX,
                        SDFColor::new(bodies::SUN_COLOR.0, bodies::SUN_COLOR.1, bodies::SUN_COLOR.2),
                    )
                    .with_emissive(bodies::SUN_EMISSIVE)
                    .with_shininess(bodies::SUN_SHININESS),
                ),
        );
        self.sun_id = Some(sun_id);

        // ── Spawn planets ────────────────────────────────────────────
        for i in 0..bodies::PLANET_COUNT {
            let (x_au, y_au) = orbit::heliocentric_position(&self.planet_elements[i], t);
            let (sx, sy) = self.au_to_screen(x_au, y_au);
            self.planet_screen_pos[i] = (sx, sy);

            let id = ctx.next_id();
            let v = &visuals[i];
            ctx.scene.spawn(
                Entity::new(id)
                    .with_tag(bodies::PLANET_NAMES[i])
                    .with_pos(Vec2::new(sx, sy))
                    .with_mesh(
                        MeshComponent::sphere(v.radius_px, SDFColor::new(v.color.0, v.color.1, v.color.2))
                            .with_emissive(v.emissive)
                            .with_shininess(v.shininess),
                    ),
            );
            self.planet_ids[i] = Some(id);
        }

        // ── Spawn moons ─────────────────────────────────────────────
        let moons = bodies::moon_data();
        for moon in &moons {
            let parent_pos = self.planet_screen_pos[moon.parent];
            let (mx, my) = Self::moon_position(parent_pos, moon.orbit_radius_px, moon.period_days, self.days, self.zoom);

            let id = ctx.next_id();
            ctx.scene.spawn(
                Entity::new(id)
                    .with_tag(moon.name)
                    .with_pos(Vec2::new(mx, my))
                    .with_mesh(
                        MeshComponent::sphere(
                            moon.radius_px,
                            SDFColor::new(moon.color.0, moon.color.1, moon.color.2),
                        )
                        .with_shininess(16.0),
                    ),
            );
            self.moon_ids.push(id);
        }

        // ── Spawn asteroids ─────────────────────────────────────────
        for i in 0..bodies::ASTEROID_COUNT {
            let (x_au, y_au) = orbit::heliocentric_position(&self.asteroid_orbits[i], t);
            let (sx, sy) = self.au_to_screen(x_au, y_au);

            // Deterministic size/color variation
            let h = bodies::asteroid_hash(i as u32 * 37 + 500);
            let frac = (h as f32) / (u32::MAX as f32);
            let base_radius = 1.0 + frac * 0.8;
            let grey = 0.3 + frac * 0.3;

            let id = ctx.next_id();
            ctx.scene.spawn(
                Entity::new(id)
                    .with_tag("asteroid")
                    .with_pos(Vec2::new(sx, sy))
                    .with_mesh(
                        MeshComponent::sphere(base_radius, SDFColor::new(grey, grey * 0.95, grey * 0.9))
                            .with_shininess(8.0),
                    ),
            );
            self.asteroid_ids.push(id);
            self.asteroid_base_radii.push(base_radius);
        }

        let (sx, sy) = self.sun_screen();
        ctx.effects.attractor = [sx, sy];
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        let dt = 1.0 / 60.0_f32;

        // ── Handle input ─────────────────────────────────────────────
        for event in input.iter() {
            match event {
                InputEvent::Custom { kind, a, b, c } => match *kind {
                    CUSTOM_SET_DAYS => {
                        self.days = *a as f64;
                    }
                    CUSTOM_SET_SPEED => {
                        self.speed = *a as f64;
                    }
                    CUSTOM_TOGGLE_PAUSE => {
                        self.paused = !self.paused;
                    }
                    CUSTOM_SELECT => {
                        let idx = *a as i32;
                        self.selected = if idx >= 0 && idx < bodies::PLANET_COUNT as i32 {
                            Some(idx as usize)
                        } else {
                            None
                        };
                    }
                    CUSTOM_ZOOM => {
                        // a = direction (+1 zoom in, -1 zoom out)
                        // b = cursor fraction X (0..1), c = cursor fraction Y (0..1)
                        let cursor_sx = *b * self.visible_w;
                        let cursor_sy = *c * self.visible_h;
                        let new_zoom = if *a > 0.0 {
                            self.zoom * ZOOM_STEP
                        } else {
                            self.zoom / ZOOM_STEP
                        };
                        self.zoom_toward(cursor_sx, cursor_sy, new_zoom);
                    }
                    CUSTOM_RESET_VIEW => {
                        self.cam_x = 0.0;
                        self.cam_y = 0.0;
                        self.zoom = 1.0;
                    }
                    CUSTOM_RESIZE => {
                        self.visible_w = *a;
                        self.visible_h = *b;
                        let _ = c; // unused
                    }
                    _ => {}
                },
                InputEvent::PointerDown { x, y } => {
                    self.dragging = true;
                    self.drag_moved = false;
                    self.drag_start = (*x, *y);
                    self.drag_cam_start = (self.cam_x, self.cam_y);
                }
                InputEvent::PointerMove { x, y } => {
                    if self.dragging {
                        let dx = *x - self.drag_start.0;
                        let dy = *y - self.drag_start.1;
                        if (dx * dx + dy * dy).sqrt() > DRAG_THRESHOLD {
                            self.drag_moved = true;
                        }
                        if self.drag_moved {
                            // Pan camera: moving pointer right → camera shifts left → content follows
                            self.cam_x = self.drag_cam_start.0 - dx as f64 / self.zoom;
                            self.cam_y = self.drag_cam_start.1 - dy as f64 / self.zoom;
                        }
                    }
                }
                InputEvent::PointerUp { x, y } => {
                    if self.dragging && !self.drag_moved {
                        // Click (not a drag) → select planet
                        self.selected = self.hit_test(Vec2::new(*x, *y));
                    }
                    self.dragging = false;
                    self.drag_moved = false;
                }
                _ => {}
            }
        }

        // ── Advance time ─────────────────────────────────────────────
        if !self.paused {
            self.days += self.speed * dt as f64;
        }

        let t = orbit::days_to_centuries(self.days);
        let z = self.zoom as f32;

        // ── Update sun position + scale ──────────────────────────────
        let (sun_x, sun_y) = self.sun_screen();
        if let Some(id) = self.sun_id {
            if let Some(entity) = ctx.scene.get_mut(id) {
                entity.pos = Vec2::new(sun_x, sun_y);
                set_sphere_radius(&mut entity.mesh, bodies::SUN_RADIUS_PX * z);
            }
        }

        // ── Update planet positions + scale ──────────────────────────
        let visuals = bodies::planet_visuals();
        for i in 0..bodies::PLANET_COUNT {
            let (x_au, y_au) = orbit::heliocentric_position(&self.planet_elements[i], t);
            let (sx, sy) = self.au_to_screen(x_au, y_au);
            self.planet_screen_pos[i] = (sx, sy);

            if let Some(id) = self.planet_ids[i] {
                if let Some(entity) = ctx.scene.get_mut(id) {
                    entity.pos = Vec2::new(sx, sy);
                    set_sphere_radius(&mut entity.mesh, visuals[i].radius_px * z);
                }
            }
        }

        // ── Update moon positions + scale ────────────────────────────
        let moons = bodies::moon_data();
        for (mi, moon) in moons.iter().enumerate() {
            if mi < self.moon_ids.len() {
                let parent_pos = self.planet_screen_pos[moon.parent];
                let (mx, my) = Self::moon_position(parent_pos, moon.orbit_radius_px, moon.period_days, self.days, self.zoom);

                if let Some(entity) = ctx.scene.get_mut(self.moon_ids[mi]) {
                    entity.pos = Vec2::new(mx, my);
                    set_sphere_radius(&mut entity.mesh, moon.radius_px * z);
                }
            }
        }

        // ── Update asteroid positions + scale ────────────────────────
        for i in 0..self.asteroid_ids.len() {
            if i < self.asteroid_orbits.len() {
                let (x_au, y_au) = orbit::heliocentric_position(&self.asteroid_orbits[i], t);
                let (sx, sy) = self.au_to_screen(x_au, y_au);

                if let Some(entity) = ctx.scene.get_mut(self.asteroid_ids[i]) {
                    entity.pos = Vec2::new(sx, sy);
                    if i < self.asteroid_base_radii.len() {
                        set_sphere_radius(&mut entity.mesh, self.asteroid_base_radii[i] * z);
                    }
                }
            }
        }

        // ── Solar effects ────────────────────────────────────────────
        ctx.effects.arcs.clear();
        self.update_solar_flares(&mut ctx.effects, dt);
        self.spawn_solar_wind(&mut ctx.effects);
        ctx.effects.attractor = [sun_x, sun_y];

        // ── Vector drawing (cleared each frame by clear_frame_data) ──
        self.draw_corona(&mut ctx.vectors);
        self.draw_orbits(&mut ctx.vectors, t);
        self.draw_saturn_rings(&mut ctx.vectors);
        self.draw_selection_ring(&mut ctx.vectors);

        // ── Emit game events ─────────────────────────────────────────
        ctx.emit_event(GameEvent {
            kind: EVENT_TIME_INFO,
            a: self.days as f32,
            b: self.speed as f32,
            c: if self.paused { 1.0 } else { 0.0 },
        });

        let (year, month, day) = orbit::days_to_date(self.days);
        ctx.emit_event(GameEvent {
            kind: EVENT_DATE_INFO,
            a: year as f32,
            b: month as f32,
            c: day as f32,
        });

        let sel_idx = self.selected.map(|i| i as f32).unwrap_or(-1.0);
        let sel_dist = self.selected.map(|i| self.planet_distance_au(i) as f32).unwrap_or(0.0);
        ctx.emit_event(GameEvent {
            kind: EVENT_SELECTION,
            a: sel_idx,
            b: sel_dist,
            c: 0.0,
        });
    }
}
