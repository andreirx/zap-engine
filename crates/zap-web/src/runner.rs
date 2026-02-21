use zap_engine::{
    Game, GameConfig, EngineContext, RenderContext,
    InputEvent, InputQueue, RenderBuffer,
    FixedTimestep, ProtocolLayout, LayerBatch,
};
use zap_engine::systems::render::build_render_buffer;
use zap_engine::systems::emitter::tick_emitters;
use zap_engine::renderer::sdf_instance::SDFBuffer;
use zap_engine::bridge::protocol::LAYER_BATCH_FLOATS;
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
    sdf_buffer: SDFBuffer,
    timestep: FixedTimestep,
    config: GameConfig,
    layout: ProtocolLayout,
    initialized: bool,
    /// Flat buffer of sound event IDs for SharedArrayBuffer reads.
    sound_buffer: Vec<u8>,
    /// Layer batch descriptors from the most recent frame.
    layer_batches: Vec<LayerBatch>,
    /// Flat f32 buffer of layer batch data for SharedArrayBuffer reads.
    /// Each batch: [layer_id, start, end, atlas_id] = 4 floats.
    layer_batch_buffer: Vec<f32>,
}

impl<G: Game> GameRunner<G> {
    pub fn new(game: G) -> Self {
        let config = game.config();
        let timestep = FixedTimestep::new(config.fixed_dt);
        let layout = ProtocolLayout::from_config(&config);

        let render_buffer = RenderBuffer::with_capacity(config.max_instances);
        let sdf_buffer = SDFBuffer::with_capacity(config.max_sdf_instances);
        let sound_buffer = Vec::with_capacity(config.max_sounds);
        let layer_batch_buffer = Vec::with_capacity(config.max_layer_batches * LAYER_BATCH_FLOATS);

        // Use with_config to wire capacity settings through all subsystems
        #[allow(unused_mut)]
        let mut ctx = EngineContext::with_config(&config);
        #[cfg(feature = "physics")]
        {
            // Physics dt = game dt / substeps (e.g., 1/60 / 4 = 1/240 for 240Hz physics)
            let physics_dt = config.fixed_dt / config.physics_substeps.max(1) as f32;
            ctx.physics.set_dt(physics_dt);
        }

        Self {
            game,
            ctx,
            input: InputQueue::new(),
            render_buffer,
            sdf_buffer,
            timestep,
            layout,
            config,
            initialized: false,
            sound_buffer,
            layer_batches: Vec::new(),
            layer_batch_buffer,
        }
    }

    /// Initialize the game. Call once after construction.
    pub fn init(&mut self) {
        self.config = self.game.config();
        self.layout = ProtocolLayout::from_config(&self.config);
        self.game.init(&mut self.ctx);
        self.initialized = true;
    }

    /// Load an asset manifest JSON string, populating the sprite registry.
    pub fn load_manifest(&mut self, json: &str) {
        match self.ctx.load_manifest(json) {
            Ok(_) => {
                // One-time verification: check if tile sprites exist
                if self.ctx.sprite("ocean_0").is_some() {
                    log::info!("Sprite registry loaded: ocean_0 found with atlas={}",
                        self.ctx.sprite("ocean_0").unwrap().atlas.0);
                } else {
                    log::warn!("Sprite registry: ocean_0 NOT FOUND");
                }
            }
            Err(e) => log::warn!("Failed to load manifest: {}", e),
        }
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

            // Run physics substeps (e.g., 4 substeps = 240Hz physics with 60Hz game updates)
            #[cfg(feature = "physics")]
            for _ in 0..self.config.physics_substeps.max(1) {
                self.ctx.step_physics();
            }

            tick_emitters(&mut self.ctx.scene, &mut self.ctx.effects, self.timestep.dt());
            self.ctx.effects.tick(self.timestep.dt());
        }

        // Drain input after update
        self.input.drain();

        // Build render buffer from entities (returns layer batch descriptors)
        self.layer_batches = build_render_buffer(self.ctx.scene.iter(), &mut self.render_buffer);

        // Serialize layer batches to flat f32 buffer for SAB
        self.layer_batch_buffer.clear();
        for batch in &self.layer_batches {
            self.layer_batch_buffer.push(batch.layer.as_u8() as f32);
            self.layer_batch_buffer.push(batch.start as f32);
            self.layer_batch_buffer.push(batch.end as f32);
            self.layer_batch_buffer.push(batch.atlas_id as f32);
        }

        // Build SDF buffer from entities with mesh components
        zap_engine::systems::sdf_render::build_sdf_buffer(self.ctx.scene.iter(), &mut self.sdf_buffer);

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

    // ---- SDF accessors ----

    pub fn sdf_instances_ptr(&self) -> *const f32 {
        self.sdf_buffer.instances_ptr()
    }

    pub fn sdf_instance_count(&self) -> u32 {
        self.sdf_buffer.instance_count() as u32
    }

    pub fn max_sdf_instances(&self) -> u32 {
        self.layout.max_sdf_instances as u32
    }

    // ---- Vector accessors ----

    #[cfg(feature = "vectors")]
    pub fn vector_vertices_ptr(&self) -> *const f32 {
        self.ctx.vectors.buffer_ptr()
    }

    #[cfg(feature = "vectors")]
    pub fn vector_vertex_count(&self) -> u32 {
        self.ctx.vectors.vertex_count() as u32
    }

    #[cfg(feature = "vectors")]
    pub fn max_vector_vertices(&self) -> u32 {
        self.layout.max_vector_vertices as u32
    }

    // ---- Bake state accessor ----

    /// Get the encoded bake state for SAB header[21].
    /// Format: baked_layers_mask | (bake_generation << 6).
    pub fn bake_state(&self) -> f32 {
        self.ctx.bake_state_encoded()
    }

    // ---- Lighting accessors ----

    pub fn lights_ptr(&self) -> *const f32 {
        self.ctx.lights.buffer_ptr()
    }

    pub fn light_count(&self) -> u32 {
        self.ctx.lights.count() as u32
    }

    pub fn max_lights(&self) -> u32 {
        self.layout.max_lights as u32
    }

    pub fn ambient_r(&self) -> f32 {
        self.ctx.lights.ambient()[0]
    }

    pub fn ambient_g(&self) -> f32 {
        self.ctx.lights.ambient()[1]
    }

    pub fn ambient_b(&self) -> f32 {
        self.ctx.lights.ambient()[2]
    }

    // ---- Layer batch accessors ----

    pub fn layer_batches_ptr(&self) -> *const f32 {
        self.layer_batch_buffer.as_ptr()
    }

    pub fn layer_batch_count(&self) -> u32 {
        self.layer_batches.len() as u32
    }

    pub fn max_layer_batches(&self) -> u32 {
        self.layout.max_layer_batches as u32
    }

    pub fn layer_batch_data_offset(&self) -> u32 {
        self.layout.layer_batch_data_offset as u32
    }
}
