use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::components::sprite::{SpriteComponent, AtlasId, BlendMode};
use zap_engine::input::queue::{InputEvent, InputQueue};
use glam::Vec2;

const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;
const SPRITE_SIZE: f32 = 40.0;
const MAX_SPRITES: usize = 64;

/// A simple bouncing-sprites demo showcasing the engine.
pub struct BasicDemo {
    velocities: Vec<Vec2>,
    next_col: f32,
}

impl BasicDemo {
    pub fn new() -> Self {
        Self {
            velocities: Vec::new(),
            next_col: 0.0,
        }
    }

    fn spawn_sprite(&mut self, ctx: &mut EngineContext, pos: Vec2) {
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
        ctx.scene.spawn(entity);

        // Random velocity
        let vx = ((id.0 * 7 + 13) % 200) as f32 - 100.0;
        let vy = ((id.0 * 11 + 17) % 200) as f32 - 100.0;
        self.velocities.push(Vec2::new(vx, vy));
    }
}

impl Game for BasicDemo {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: 1.0 / 60.0,
            world_width: WORLD_W,
            world_height: WORLD_H,
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        // Spawn initial sprites in a grid
        for i in 0..8 {
            let x = 100.0 + (i % 4) as f32 * 150.0;
            let y = 150.0 + (i / 4) as f32 * 150.0;
            self.spawn_sprite(ctx, Vec2::new(x, y));
        }
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        // Handle input: spawn sprite on click
        for event in input.iter() {
            if let InputEvent::PointerDown { x, y } = event {
                self.spawn_sprite(ctx, Vec2::new(*x, *y));
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

        // Bounce sprites off walls
        let half = SPRITE_SIZE / 2.0;
        let mut idx = 0;
        for entity in ctx.scene.iter_mut() {
            if idx >= self.velocities.len() {
                break;
            }
            let vel = &mut self.velocities[idx];

            entity.pos += *vel * (1.0 / 60.0);

            // Bounce off walls
            if entity.pos.x - half < 0.0 {
                entity.pos.x = half;
                vel.x = vel.x.abs();
            } else if entity.pos.x + half > WORLD_W {
                entity.pos.x = WORLD_W - half;
                vel.x = -vel.x.abs();
            }
            if entity.pos.y - half < 0.0 {
                entity.pos.y = half;
                vel.y = vel.y.abs();
            } else if entity.pos.y + half > WORLD_H {
                entity.pos.y = WORLD_H - half;
                vel.y = -vel.y.abs();
            }

            // Slowly rotate
            entity.rotation += 0.5 * (1.0 / 60.0);

            idx += 1;
        }

        // Set particle attractor to center
        ctx.effects.attractor = [WORLD_W / 2.0, WORLD_H];
    }
}
