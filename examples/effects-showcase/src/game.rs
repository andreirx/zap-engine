use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::components::sprite::{SpriteComponent, AtlasId, BlendMode};
use zap_engine::components::emitter::{EmitterComponent, EmissionMode, ParticleColorMode};
use zap_engine::components::layer::RenderLayer;
use zap_engine::systems::effects::{SegmentColor, ParticleBlend};
use zap_engine::systems::lighting::PointLight;
use zap_engine::systems::visibility::VisibilityInterpolation;
use zap_engine::input::queue::InputQueue;
use glam::Vec2;

const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;
const VIS_COLS: u32 = 40;
const VIS_ROWS: u32 = 30;
const CELL_W: f32 = WORLD_W / VIS_COLS as f32;
const CELL_H: f32 = WORLD_H / VIS_ROWS as f32;
const REVEAL_RADIUS: f32 = 6.0; // cells

/// Demonstrates per-sprite blend modes, alpha/additive particles, and visibility masking.
pub struct EffectsShowcase {
    viewpoint: Vec2,
    time: f32,
    orb_ids: Vec<EntityId>,
    ui_ids: Vec<EntityId>,
    explosions: Vec<Explosion>,
}

struct Explosion {
    pos: Vec2,
    age: f32,
    light_intensity: f32,
}

impl EffectsShowcase {
    pub fn new() -> Self {
        Self {
            viewpoint: Vec2::new(WORLD_W / 2.0, WORLD_H / 2.0),
            time: 0.0,
            orb_ids: Vec::new(),
            ui_ids: Vec::new(),
            explosions: Vec::new(),
        }
    }
}

impl Game for EffectsShowcase {
    fn config(&self) -> GameConfig {
        GameConfig {
            world_width: WORLD_W,
            world_height: WORLD_H,
            max_instances: 512,
            max_effects_vertices: 32768,
            visibility_cols: VIS_COLS,
            visibility_rows: VIS_ROWS,
            visibility_interpolation: VisibilityInterpolation::Linear,
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        // --- Background: scattered alpha-blend terrain sprites ---
        for row in 0..6 {
            for col in 0..8 {
                let id = ctx.next_id();
                ctx.scene.spawn(
                    Entity::new(id)
                        .with_layer(RenderLayer::Background)
                        .with_pos(Vec2::new(col as f32 * 100.0 + 50.0, row as f32 * 100.0 + 50.0))
                        .with_scale(Vec2::splat(100.0))
                        .with_sprite(SpriteComponent {
                            atlas: AtlasId(0),
                            col: (col % 4) as f32,
                            row: 0.0,
                            cell_span: 1.0,
                            alpha: 0.3,
                            blend: BlendMode::Alpha,
                        }),
                );
            }
        }

        // --- Additive-blend orbs (glowing energy) ---
        for i in 0..5 {
            let id = ctx.next_id();
            let angle = i as f32 * std::f32::consts::TAU / 5.0;
            let x = WORLD_W / 2.0 + angle.cos() * 150.0;
            let y = WORLD_H / 2.0 + angle.sin() * 150.0;
            ctx.scene.spawn(
                Entity::new(id)
                    .with_layer(RenderLayer::Objects)
                    .with_pos(Vec2::new(x, y))
                    .with_scale(Vec2::splat(40.0))
                    .with_sprite(SpriteComponent {
                        atlas: AtlasId(0),
                        col: 0.0,
                        row: 1.0,
                        cell_span: 1.0,
                        alpha: 1.5, // HDR glow
                        blend: BlendMode::Additive,
                    }),
            );
            self.orb_ids.push(id);
        }

        // --- Continuous smoke emitter (alpha particles on VFX layer) ---
        let smoke_id = ctx.next_id();
        ctx.scene.spawn(
            Entity::new(smoke_id)
                .with_pos(Vec2::new(200.0, 400.0))
                .with_emitter(
                    EmitterComponent::new()
                        .with_mode(EmissionMode::Continuous)
                        .with_rate(15.0)
                        .with_speed_range(1.0, 3.0)
                        .with_width(8.0)
                        .with_lifetime(2.0)
                        .with_drag(0.08)
                        .with_attract_strength(0.0)
                        .with_speed_factor(0.5)
                        .with_color_mode(ParticleColorMode::Palette(vec![
                            SegmentColor::White,
                            SegmentColor::SkyBlue,
                        ]))
                        .with_blend(ParticleBlend::Alpha)
                        .with_particle_layer(RenderLayer::VFX),
                ),
        );

        // --- Continuous spark emitter (additive particles, classic glow) ---
        let spark_id = ctx.next_id();
        ctx.scene.spawn(
            Entity::new(spark_id)
                .with_pos(Vec2::new(600.0, 400.0))
                .with_emitter(
                    EmitterComponent::new()
                        .with_mode(EmissionMode::Continuous)
                        .with_rate(20.0)
                        .with_speed_range(3.0, 8.0)
                        .with_width(3.0)
                        .with_lifetime(1.0)
                        .with_color_mode(ParticleColorMode::Palette(vec![
                            SegmentColor::Orange,
                            SegmentColor::Yellow,
                            SegmentColor::Red,
                        ]))
                        .with_blend(ParticleBlend::Additive)
                        .with_particle_layer(RenderLayer::VFX),
                ),
        );

        // --- UI label (stays visible through fog) ---
        let ui_id = ctx.next_id();
        ctx.scene.spawn(
            Entity::new(ui_id)
                .with_layer(RenderLayer::UI)
                .with_pos(Vec2::new(WORLD_W / 2.0, 30.0))
                .with_scale(Vec2::splat(48.0))
                .with_sprite(SpriteComponent {
                    atlas: AtlasId(0),
                    col: 0.0,
                    row: 2.0,
                    cell_span: 1.0,
                    alpha: 1.0,
                    blend: BlendMode::Alpha,
                }),
        );
        self.ui_ids.push(ui_id);

        // --- Lighting: ambient dim + viewpoint light ---
        ctx.lights.set_ambient(0.15, 0.15, 0.2);

        log::info!("EffectsShowcase: initialized with visibility {}x{}", VIS_COLS, VIS_ROWS);
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        let dt = 1.0 / 60.0;
        self.time += dt;

        // --- Move viewpoint with pointer ---
        for event in input.iter() {
            match event {
                InputEvent::PointerMove { x, y } | InputEvent::PointerDown { x, y } => {
                    self.viewpoint = Vec2::new(*x, *y);
                }
                InputEvent::PointerUp { x, y } => {
                    // Spawn explosion at click position
                    let pos = Vec2::new(*x, *y);

                    // Fire core: additive burst
                    ctx.effects.spawn_particles(
                        [pos.x, pos.y], 30, 12.0, 5.0, 0.8,
                    );

                    // Smoke cloud: alpha particles on VFX layer
                    ctx.effects.spawn_alpha_particles(
                        [pos.x, pos.y], 20, 4.0, 10.0, 2.5, RenderLayer::VFX,
                    );

                    // Add explosion light
                    self.explosions.push(Explosion {
                        pos,
                        age: 0.0,
                        light_intensity: 3.0,
                    });
                }
                _ => {}
            }
        }

        // --- Animate orbs in a circle ---
        for (i, &id) in self.orb_ids.iter().enumerate() {
            if let Some(entity) = ctx.scene.get_mut(id) {
                let base_angle = i as f32 * std::f32::consts::TAU / 5.0;
                let angle = base_angle + self.time * 0.5;
                entity.pos.x = WORLD_W / 2.0 + angle.cos() * 150.0;
                entity.pos.y = WORLD_H / 2.0 + angle.sin() * 150.0;
                entity.rotation = self.time * 2.0;
            }
        }

        // --- Update explosions ---
        self.explosions.retain_mut(|exp| {
            exp.age += dt;
            exp.light_intensity *= 0.92; // Decay
            exp.age < 3.0
        });

        // --- Lighting ---
        ctx.lights.clear();
        ctx.lights.set_ambient(0.15, 0.15, 0.2);

        // Viewpoint light
        ctx.lights.add(
            PointLight::new(self.viewpoint, [1.0, 0.95, 0.8], 1.5, 250.0)
        );

        // Explosion lights
        for exp in &self.explosions {
            if exp.light_intensity > 0.05 {
                ctx.lights.add(
                    PointLight::new(exp.pos, [1.0, 0.6, 0.2], exp.light_intensity, 180.0)
                );
            }
        }

        // --- Visibility mask: circular reveal around viewpoint ---
        if let Some(vis) = &mut ctx.visibility {
            vis.clear();
            // Convert world-space viewpoint to grid coordinates
            let grid_x = self.viewpoint.x / CELL_W;
            let grid_y = self.viewpoint.y / CELL_H;
            vis.set_circle(grid_x, grid_y, REVEAL_RADIUS, 255);
            // Slightly larger dim circle for explored-but-distant effect
            vis.set_circle(grid_x, grid_y, REVEAL_RADIUS * 1.8, 80);
            // Re-stamp the inner circle at full brightness (set_circle overwrites)
            vis.set_circle(grid_x, grid_y, REVEAL_RADIUS, 255);
        }
    }
}
