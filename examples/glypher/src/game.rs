use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::input::queue::InputQueue;

const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;

pub struct Glypher;

impl Glypher {
    pub fn new() -> Self {
        Self
    }
}

impl Game for Glypher {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: 1.0 / 60.0,
            world_width: WORLD_W,
            world_height: WORLD_H,
            ..GameConfig::default()
        }
    }

    fn init(&mut self, _ctx: &mut EngineContext) {
        log::info!("Glypher initialized");
    }

    fn update(&mut self, _ctx: &mut EngineContext, _input: &InputQueue) {
        // Game logic goes here
    }
}
