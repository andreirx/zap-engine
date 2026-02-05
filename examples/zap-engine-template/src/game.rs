use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::components::sprite::{SpriteComponent, AtlasId, BlendMode};
use zap_engine::input::queue::InputQueue;
use glam::Vec2;

const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;

/// Minimal "Hello World" game — spawns a spinning sprite in the center.
/// Copy this template and build your game from here.
pub struct HelloGame {
    sprite_id: Option<EntityId>,
}

impl HelloGame {
    pub fn new() -> Self {
        Self { sprite_id: None }
    }
}

impl Game for HelloGame {
    fn config(&self) -> GameConfig {
        GameConfig {
            world_width: WORLD_W,
            world_height: WORLD_H,
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        // Spawn a single sprite in the center of the world
        let id = ctx.next_id();
        ctx.scene.spawn(
            Entity::new(id)
                .with_pos(Vec2::new(WORLD_W / 2.0, WORLD_H / 2.0))
                .with_scale(Vec2::splat(64.0))
                .with_sprite(SpriteComponent {
                    atlas: AtlasId(0),
                    col: 0.0,
                    row: 0.0,
                    cell_span: 1.0,
                    alpha: 1.0,
                    blend: BlendMode::Alpha,
                }),
        );
        self.sprite_id = Some(id);
        log::info!("HelloGame: spawned sprite at center");
    }

    fn update(&mut self, ctx: &mut EngineContext, _input: &InputQueue) {
        // Rotate the sprite each frame
        if let Some(id) = self.sprite_id {
            if let Some(entity) = ctx.scene.get_mut(id) {
                entity.rotation += 0.02;
            }
        }
    }
}
