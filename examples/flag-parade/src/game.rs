use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::input::queue::{InputEvent, InputQueue};
use zap_engine::components::mesh::{MeshComponent, SDFColor, SDFShape};
use glam::{Vec2, Vec3};

use crate::flags;

const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;
const FIXED_DT: f32 = 1.0 / 60.0;

const COLS: usize = 24;
const ROWS: usize = 16;
const SPHERE_RADIUS: f32 = 11.0;
const SPACING: f32 = 25.0;

// ── 3D Cloth physics ─────────────────────────────────────────────────
// The flag exists in 3D: X = right, Y = down, Z = toward viewer.
// We render by projecting (x, y) to screen and using z for depth shading.
const REST_H: f32 = SPACING;
const REST_V: f32 = SPACING;
const REST_DIAG: f32 = SPACING * 1.4142135; // sqrt(2)

const CONSTRAINT_ITERS: usize = 8;
const DAMPING: f32 = 0.008;

// Forces — Verlet uses accel * dt², dt² ≈ 0.000278 at 60fps
const GRAVITY: f32 = 2500.0;
// Horizontal wind keeps the flag extended to the right
const WIND_X: f32 = 1800.0;
// Z-axis wind creates the billowing effect (perpendicular to screen)
const WIND_Z_BASE: f32 = 8000.0;
const WIND_Z_VARY: f32 = 5000.0;
// Per-particle turbulence for organic randomness
const TURB_STRENGTH: f32 = 3500.0;

// Depth shading (driven by z position)
const BRIGHTNESS_MIN: f32 = 0.45;
const BRIGHTNESS_RANGE: f32 = 0.55;
const SCALE_RANGE: f32 = 0.15;
const EMISSIVE_BASE: f32 = 0.03;
const EMISSIVE_PEAK: f32 = 0.25;
const SHININESS: f32 = 48.0;

/// Custom event: select flag (kind=1, a=flag_index)
const CUSTOM_SELECT_FLAG: u32 = 1;

/// Grid origin — centered in world
const ORIGIN_X: f32 = (WORLD_W - (COLS - 1) as f32 * SPACING) / 2.0;
const ORIGIN_Y: f32 = (WORLD_H - (ROWS - 1) as f32 * SPACING) / 2.0;

pub struct FlagParade {
    time: f32,
    current_flag: usize,
    ids: Vec<EntityId>,
    // 3D Verlet cloth state
    pos: Vec<Vec3>,
    old_pos: Vec<Vec3>,
    rest_pos: Vec<Vec3>,
}

impl FlagParade {
    pub fn new() -> Self {
        let cap = COLS * ROWS;
        Self {
            time: 0.0,
            current_flag: 0,
            ids: Vec::with_capacity(cap),
            pos: Vec::with_capacity(cap),
            old_pos: Vec::with_capacity(cap),
            rest_pos: Vec::with_capacity(cap),
        }
    }

    fn grid_pos(col: usize, row: usize) -> Vec3 {
        Vec3::new(
            ORIGIN_X + col as f32 * SPACING,
            ORIGIN_Y + row as f32 * SPACING,
            0.0, // z=0 is the rest plane
        )
    }

    fn idx(col: usize, row: usize) -> usize {
        row * COLS + col
    }

    /// Satisfy a 3D distance constraint between two particles.
    fn satisfy_constraint(&mut self, i: usize, j: usize, rest_len: f32) {
        let delta = self.pos[j] - self.pos[i];
        let dist = delta.length();
        if dist < 0.001 {
            return;
        }
        let correction = delta * (1.0 - rest_len / dist) * 0.5;
        let i_fixed = (i % COLS) == 0;
        let j_fixed = (j % COLS) == 0;

        if i_fixed && j_fixed {
            return;
        } else if i_fixed {
            self.pos[j] -= correction;
        } else if j_fixed {
            self.pos[i] += correction;
        } else {
            self.pos[i] += correction;
            self.pos[j] -= correction;
        }
    }

    /// Run one step of 3D Verlet cloth simulation.
    fn simulate(&mut self) {
        let t = self.time;
        let dt2 = FIXED_DT * FIXED_DT;

        // Global Z wind — varies over time with multiple frequencies
        let wind_z = WIND_Z_BASE
            + (t * 0.8).sin() * WIND_Z_VARY
            + (t * 2.1).sin() * WIND_Z_VARY * 0.3
            + (t * 0.35).sin() * WIND_Z_VARY * 0.5;

        // ── Verlet integration (3D) ───────────────────────────────────
        for row in 0..ROWS {
            for col in 0..COLS {
                let i = Self::idx(col, row);
                if col == 0 {
                    continue; // left column pinned to pole
                }

                let cf = col as f32;
                let rf = row as f32;

                // Wind grows stronger further from the pole
                let wind_factor = (cf / (COLS - 1) as f32).sqrt();

                // Z turbulence: traveling wave from pole to tip + per-row variation
                let phase = cf * 0.4 - t * 4.0;
                let turb_z = (phase.sin() + (rf * 0.6 + t * 2.5).sin())
                    * TURB_STRENGTH;

                // Y turbulence (smaller vertical perturbation)
                let turb_y = (cf * 0.3 + t * 1.8).sin()
                    * (rf * 0.5 + t * 2.2).cos()
                    * TURB_STRENGTH * 0.2;

                // X turbulence (slight horizontal variation)
                let turb_x = (cf * 0.7 + t * 1.5).sin()
                    * (rf * 0.4 + t * 2.8).cos()
                    * TURB_STRENGTH * 0.15;

                let accel = Vec3::new(
                    (WIND_X + turb_x) * wind_factor,         // extends flag right
                    GRAVITY + turb_y,                         // gravity + turbulence
                    (wind_z + turb_z) * wind_factor,          // Z billowing
                );

                let vel = (self.pos[i] - self.old_pos[i]) * (1.0 - DAMPING);
                self.old_pos[i] = self.pos[i];
                self.pos[i] = self.pos[i] + vel + accel * dt2;
            }
        }

        // ── 3D constraint satisfaction ────────────────────────────────
        // More iterations = stiffer cloth (less stretching)
        for _ in 0..CONSTRAINT_ITERS {
            // Horizontal (structural)
            for row in 0..ROWS {
                for col in 0..COLS - 1 {
                    let i = Self::idx(col, row);
                    let j = Self::idx(col + 1, row);
                    self.satisfy_constraint(i, j, REST_H);
                }
            }
            // Vertical (structural)
            for row in 0..ROWS - 1 {
                for col in 0..COLS {
                    let i = Self::idx(col, row);
                    let j = Self::idx(col, row + 1);
                    self.satisfy_constraint(i, j, REST_V);
                }
            }
            // Diagonal (shear — prevents parallelogram deformation)
            for row in 0..ROWS - 1 {
                for col in 0..COLS - 1 {
                    let i = Self::idx(col, row);
                    self.satisfy_constraint(i, Self::idx(col + 1, row + 1), REST_DIAG);
                    self.satisfy_constraint(
                        Self::idx(col + 1, row),
                        Self::idx(col, row + 1),
                        REST_DIAG,
                    );
                }
            }
            // Re-pin left column to the pole
            for row in 0..ROWS {
                let i = Self::idx(0, row);
                self.pos[i] = self.rest_pos[i];
            }
        }
    }

    /// Depth from the Z axis — positive z = closer to viewer = brighter.
    fn depth_at(&self, col: usize, row: usize) -> f32 {
        let i = Self::idx(col, row);
        (self.pos[i].z / (SPACING * 3.0)).clamp(-1.0, 1.0)
    }
}

impl Game for FlagParade {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: FIXED_DT,
            world_width: WORLD_W,
            world_height: WORLD_H,
            max_sdf_instances: 512,
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        for row in 0..ROWS {
            for col in 0..COLS {
                let id = ctx.next_id();
                let rest = Self::grid_pos(col, row);
                let (r, g, b) = flags::flag_color(self.current_flag, col, row, COLS, ROWS);

                let entity = Entity::new(id)
                    .with_pos(Vec2::new(rest.x, rest.y))
                    .with_mesh(
                        MeshComponent::sphere(SPHERE_RADIUS, SDFColor::new(r, g, b))
                            .with_shininess(SHININESS),
                    );

                ctx.scene.spawn(entity);
                self.ids.push(id);
                self.pos.push(rest);
                self.old_pos.push(rest);
                self.rest_pos.push(rest);
            }
        }
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        for event in input.iter() {
            if let InputEvent::Custom { kind, a, .. } = event {
                if *kind == CUSTOM_SELECT_FLAG {
                    let idx = *a as usize;
                    if idx < flags::FLAG_COUNT {
                        self.current_flag = idx;
                    }
                }
            }
        }

        self.time += FIXED_DT;

        // Run 3D cloth physics
        self.simulate();

        // Update rendering — project 3D to 2D
        for row in 0..ROWS {
            for col in 0..COLS {
                let i = Self::idx(col, row);
                let id = self.ids[i];

                let depth = self.depth_at(col, row);
                let brightness = BRIGHTNESS_MIN + BRIGHTNESS_RANGE * depth;
                let scale_factor = 1.0 + SCALE_RANGE * depth;
                let emissive = EMISSIVE_BASE + EMISSIVE_PEAK * depth.max(0.0);

                let (r, g, b) = flags::flag_color(self.current_flag, col, row, COLS, ROWS);
                let color = SDFColor::new(r * brightness, g * brightness, b * brightness);

                if let Some(entity) = ctx.scene.get_mut(id) {
                    // Project: use x,y for screen position, z drives depth shading
                    entity.pos = Vec2::new(self.pos[i].x, self.pos[i].y);
                    if let Some(ref mut mesh) = entity.mesh {
                        mesh.color = color;
                        mesh.emissive = emissive;
                        mesh.shape = SDFShape::Sphere {
                            radius: SPHERE_RADIUS * scale_factor,
                        };
                    }
                }
            }
        }
    }
}
