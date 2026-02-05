use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::components::sprite::{SpriteComponent, AtlasId, BlendMode};
use zap_engine::input::queue::{InputEvent, InputQueue};
use glam::Vec2;

const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;
const SPRITE_SIZE: f32 = 40.0;
const MAX_SPRITES: usize = 64;
const WALL_THICKNESS: f32 = 20.0;
const BALL_RADIUS: f32 = SPRITE_SIZE / 2.0;
const BALL_RESTITUTION: f32 = 0.8;

/// A physics-driven bouncing-sprites demo showcasing the engine.
pub struct BasicDemo {
    next_col: f32,
}

impl BasicDemo {
    pub fn new() -> Self {
        Self {
            next_col: 0.0,
        }
    }

    fn spawn_physics_sprite(&mut self, ctx: &mut EngineContext, pos: Vec2) {
        if ctx.scene.len() >= MAX_SPRITES {
            return;
        }
        let id = ctx.next_id();
        let col = self.next_col;
        self.next_col = (self.next_col + 1.0) % 4.0;

        let entity = Entity::new(id)
            .with_pos(pos)
            .with_scale(Vec2::splat(SPRITE_SIZE))
            .with_sprite(SpriteComponent {
                atlas: AtlasId(0),
                col,
                row: 0.0,
                cell_span: 1.0,
                alpha: 1.0,
                blend: BlendMode::Alpha,
            });

        // Random velocity derived from entity ID
        let vx = ((id.0 * 7 + 13) % 200) as f32 - 100.0;
        let vy = ((id.0 * 11 + 17) % 200) as f32 - 100.0;

        let desc = BodyDesc::dynamic(ColliderDesc::Ball { radius: BALL_RADIUS })
            .with_position(pos)
            .with_velocity(Vec2::new(vx, vy));

        let material = ColliderMaterial {
            restitution: BALL_RESTITUTION,
            friction: 0.3,
            density: 1.0,
        };

        ctx.spawn_with_body(entity, desc, material);
    }

    fn spawn_wall(ctx: &mut EngineContext, pos: Vec2, half_w: f32, half_h: f32) {
        let id = ctx.next_id();
        let entity = Entity::new(id)
            .with_tag("wall")
            .with_pos(pos)
            .with_scale(Vec2::new(half_w * 2.0, half_h * 2.0));
        // Invisible wall (no sprite) — fixed body
        let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
            half_width: half_w,
            half_height: half_h,
        })
        .with_position(pos);

        ctx.spawn_with_body(entity, desc, ColliderMaterial::default());
    }
}

impl Game for BasicDemo {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: 1.0 / 60.0,
            world_width: WORLD_W,
            world_height: WORLD_H,
            gravity: Vec2::new(0.0, 981.0),
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        // Spawn 4 invisible walls forming a boundary box
        let half_w = WORLD_W / 2.0;
        let half_h = WORLD_H / 2.0;

        // Top wall
        BasicDemo::spawn_wall(
            ctx,
            Vec2::new(half_w, -WALL_THICKNESS / 2.0),
            half_w + WALL_THICKNESS,
            WALL_THICKNESS / 2.0,
        );
        // Bottom wall
        BasicDemo::spawn_wall(
            ctx,
            Vec2::new(half_w, WORLD_H + WALL_THICKNESS / 2.0),
            half_w + WALL_THICKNESS,
            WALL_THICKNESS / 2.0,
        );
        // Left wall
        BasicDemo::spawn_wall(
            ctx,
            Vec2::new(-WALL_THICKNESS / 2.0, half_h),
            WALL_THICKNESS / 2.0,
            half_h + WALL_THICKNESS,
        );
        // Right wall
        BasicDemo::spawn_wall(
            ctx,
            Vec2::new(WORLD_W + WALL_THICKNESS / 2.0, half_h),
            WALL_THICKNESS / 2.0,
            half_h + WALL_THICKNESS,
        );

        // Spawn initial sprites in a grid
        for i in 0..8 {
            let x = 100.0 + (i % 4) as f32 * 150.0;
            let y = 150.0 + (i / 4) as f32 * 150.0;
            self.spawn_physics_sprite(ctx, Vec2::new(x, y));
        }
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        // Handle input: spawn sprite on click
        for event in input.iter() {
            if let InputEvent::PointerDown { x, y } = event {
                self.spawn_physics_sprite(ctx, Vec2::new(*x, *y));
                // Spawn a particle burst at click position
                ctx.effects.spawn_particles(
                    [*x, *y],
                    5,
                    8.0,
                    3.0,
                    1.5,
                );
                ctx.emit_sound(SoundEvent(0));
            }
        }

        // Collect collision midpoints first (avoids borrow conflict with effects)
        let spark_positions: Vec<Vec2> = ctx
            .collisions()
            .iter()
            .filter(|c| c.started)
            .filter_map(|c| {
                let a = ctx.scene.get(c.entity_a).map(|e| e.pos)?;
                let b = ctx.scene.get(c.entity_b).map(|e| e.pos)?;
                Some((a + b) / 2.0)
            })
            .collect();

        // Spawn sparks at collision points
        for mid in spark_positions {
            ctx.effects.spawn_particles(
                [mid.x, mid.y],
                3,
                4.0,
                2.0,
                0.8,
            );
        }

        // Set particle attractor to center-bottom
        ctx.effects.attractor = [WORLD_W / 2.0, WORLD_H];
    }
}
