use zap_engine::{
    Game, GameConfig, EngineContext, RenderContext,
    InputEvent, InputQueue, RenderBuffer,
    FixedTimestep, ProtocolLayout,
};
use zap_engine::systems::render::build_render_buffer;
/// Generic game runner that wires up the engine loop.
///
/// Each concrete game (e.g., `basic-demo`) creates a `thread_local!` GameRunner
/// and exports free functions via `#[wasm_bindgen]`, because wasm-bindgen
/// cannot export generic structs directly.
pub struct GameRunner<G: Game> {
    game: G,
    ctx: EngineContext,
    input: InputQueue,
    render_buffer: RenderBuffer,
    timestep: FixedTimestep,
    config: GameConfig,
    layout: ProtocolLayout,
    initialized: bool,
    /// Flat buffer of sound event IDs for SharedArrayBuffer reads.
    sound_buffer: Vec<u8>,
}

impl<G: Game> GameRunner<G> {
    pub fn new(game: G) -> Self {
        let config = game.config();
        let timestep = FixedTimestep::new(config.fixed_dt);
        let layout = ProtocolLayout::from_config(&config);

        let render_buffer = RenderBuffer::with_capacity(config.max_instances);
        let sound_buffer = Vec::with_capacity(config.max_sounds);

        Self {
            game,
            ctx: EngineContext::new(),
            input: InputQueue::new(),
            render_buffer,
            timestep,
            layout,
            config,
            initialized: false,
            sound_buffer,
        }
    }

    /// Initialize the game. Call once after construction.
    pub fn init(&mut self) {
        self.config = self.game.config();
        self.layout = ProtocolLayout::from_config(&self.config);
        self.game.init(&mut self.ctx);
        self.initialized = true;
    }

    /// Push an input event into the queue.
    pub fn push_input(&mut self, event: InputEvent) {
        self.input.push(event);
    }

    /// Run one frame tick: update game, build render buffer, run effects.
    pub fn tick(&mut self, dt: f32) {
        if !self.initialized {
            return;
        }

        // Clear per-frame transient data
        self.ctx.clear_frame_data();

        // Fixed timestep accumulation
        let steps = self.timestep.accumulate(dt);
        for _ in 0..steps {
            self.game.update(&mut self.ctx, &self.input);
            self.ctx.effects.tick(self.timestep.dt());
        }

        // Drain input after update
        self.input.drain();

        // Build render buffer from entities
        build_render_buffer(self.ctx.scene.iter(), &mut self.render_buffer);

        // Allow game to add custom render commands
        {
            let mut render_ctx = RenderContext {
                render_buffer: &mut self.render_buffer,
            };
            self.game.render(&mut render_ctx);
        }

        // Rebuild effects buffer
        self.ctx.effects.rebuild_effects_buffer();

        // Pack sound events into flat buffer
        self.sound_buffer.clear();
        for sound in &self.ctx.sounds {
            self.sound_buffer.push(sound.0 as u8);
        }
    }

    // ---- Pointer accessors for SharedArrayBuffer reads ----

    pub fn instances_ptr(&self) -> *const f32 {
        self.render_buffer.instances_ptr()
    }

    pub fn instance_count(&self) -> u32 {
        self.render_buffer.instance_count()
    }

    pub fn effects_ptr(&self) -> *const f32 {
        self.ctx.effects.effects_buffer_ptr()
    }

    pub fn effects_vertex_count(&self) -> u32 {
        self.ctx.effects.effects_vertex_count() as u32
    }

    pub fn sound_events_ptr(&self) -> *const u8 {
        self.sound_buffer.as_ptr()
    }

    pub fn sound_events_len(&self) -> u32 {
        self.sound_buffer.len() as u32
    }

    pub fn game_events_ptr(&self) -> *const f32 {
        self.ctx.events.as_ptr() as *const f32
    }

    pub fn game_events_len(&self) -> u32 {
        self.ctx.events.len() as u32
    }

    pub fn world_width(&self) -> f32 {
        self.config.world_width
    }

    pub fn world_height(&self) -> f32 {
        self.config.world_height
    }

    pub fn atlas_split(&self) -> u32 {
        self.render_buffer.atlas_split
    }

    // ---- Capacity accessors (read by TypeScript via wasm_bindgen exports) ----

    pub fn max_instances(&self) -> u32 {
        self.layout.max_instances as u32
    }

    pub fn max_effects_vertices(&self) -> u32 {
        self.layout.max_effects_vertices as u32
    }

    pub fn max_sounds(&self) -> u32 {
        self.layout.max_sounds as u32
    }

    pub fn max_events(&self) -> u32 {
        self.layout.max_events as u32
    }

    pub fn buffer_total_floats(&self) -> u32 {
        self.layout.buffer_total_floats as u32
    }
}
