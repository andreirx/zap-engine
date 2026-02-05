use crate::core::scene::Scene;
use crate::api::types::{EntityId, SoundEvent, GameEvent};
use crate::input::queue::InputQueue;
use crate::renderer::instance::RenderBuffer;
use crate::systems::effects::EffectsState;

/// Configuration for the engine, provided by the game.
#[derive(Debug, Clone)]
pub struct GameConfig {
    /// Fixed timestep in seconds (default: 1/60).
    pub fixed_dt: f32,
    /// World width in game units.
    pub world_width: f32,
    /// World height in game units.
    pub world_height: f32,
    /// Maximum number of render instances (default: 512).
    pub max_instances: usize,
    /// Maximum number of effects vertices (default: 16384).
    pub max_effects_vertices: usize,
    /// Maximum number of sound events per frame (default: 32).
    pub max_sounds: usize,
    /// Maximum number of game events per frame (default: 32).
    pub max_events: usize,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            fixed_dt: 1.0 / 60.0,
            world_width: 800.0,
            world_height: 600.0,
            max_instances: 512,
            max_effects_vertices: 16384,
            max_sounds: 32,
            max_events: 32,
        }
    }
}

/// The core contract every game must fulfill.
pub trait Game {
    /// Return engine configuration. Called once before init.
    fn config(&self) -> GameConfig {
        GameConfig::default()
    }

    /// Setup initial state, spawn entities, configure the scene.
    fn init(&mut self, ctx: &mut EngineContext);

    /// The game loop tick. Apply forces, check win conditions, spawn/despawn entities.
    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue);

    /// Optional read-only render pass for custom render commands.
    fn render(&self, _ctx: &mut RenderContext) {}
}

/// Mutable access to engine state, passed to Game::init and Game::update.
pub struct EngineContext {
    pub scene: Scene,
    pub effects: EffectsState,
    pub sounds: Vec<SoundEvent>,
    pub events: Vec<GameEvent>,
    next_id: u32,
}

impl EngineContext {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            effects: EffectsState::new(42),
            sounds: Vec::new(),
            events: Vec::new(),
            next_id: 1,
        }
    }

    /// Generate the next unique entity ID.
    pub fn next_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Emit a sound event to be forwarded to TypeScript.
    pub fn emit_sound(&mut self, event: SoundEvent) {
        self.sounds.push(event);
    }

    /// Emit a game event to be forwarded to TypeScript.
    pub fn emit_event(&mut self, event: GameEvent) {
        self.events.push(event);
    }

    /// Clear per-frame transient data (sounds, events).
    pub fn clear_frame_data(&mut self) {
        self.sounds.clear();
        self.events.clear();
    }
}

impl Default for EngineContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Render context for optional custom render commands.
pub struct RenderContext<'a> {
    pub render_buffer: &'a mut RenderBuffer,
}
