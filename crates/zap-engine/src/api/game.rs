use crate::core::scene::Scene;
use crate::api::types::{EntityId, SoundEvent, GameEvent};
use crate::input::queue::InputQueue;
use crate::renderer::instance::RenderBuffer;
use crate::renderer::camera::Camera2D;
use crate::systems::effects::EffectsState;
use crate::systems::text::{FontConfig, build_text_entities, despawn_text};
use crate::assets::manifest::AssetManifest;
use crate::assets::registry::SpriteRegistry;
use crate::bridge::protocol::{DEFAULT_MAX_LAYER_BATCHES, DEFAULT_MAX_LIGHTS};
use crate::components::layer::RenderLayer;
use crate::components::sprite::SpriteComponent;
use crate::systems::lighting::LightState;
use glam::Vec2;
#[cfg(feature = "physics")]
use crate::core::physics::{
    PhysicsWorld, BodyDesc, ColliderMaterial, CollisionPair,
    JointHandle, JointDesc,
};
#[cfg(feature = "physics")]
use crate::components::entity::Entity;
#[cfg(feature = "vectors")]
use crate::systems::vector::VectorState;

// ============================================================================
// GameConfig
// ============================================================================

/// Configuration for the engine, provided by the game.
#[derive(Debug, Clone)]
pub struct GameConfig {
    /// Fixed timestep in seconds (default: 1/60).
    pub fixed_dt: f32,
    /// World width in game units.
    pub world_width: f32,
    /// World height in game units.
    pub world_height: f32,
    /// Maximum number of entities in the scene (default: 512).
    pub max_entities: usize,
    /// Maximum number of render instances (default: 512).
    pub max_instances: usize,
    /// Maximum number of effects vertices (default: 16384).
    pub max_effects_vertices: usize,
    /// Maximum number of sound events per frame (default: 32).
    pub max_sounds: usize,
    /// Maximum number of game events per frame (default: 32).
    pub max_events: usize,
    /// Maximum number of SDF instances (default: 128).
    pub max_sdf_instances: usize,
    /// Maximum number of vector vertices (default: 16384).
    #[cfg(feature = "vectors")]
    pub max_vector_vertices: usize,
    /// Maximum number of layer batches (default: 6, one per RenderLayer).
    pub max_layer_batches: usize,
    /// Maximum number of point lights (default: 64).
    pub max_lights: usize,
    /// Seed for effects RNG (default: 42). Change for different random sequences.
    pub effects_seed: u64,
    /// Gravity vector for physics simulation. Default: zero (no gravity).
    /// For Y-down coordinate systems, use positive Y for downward gravity.
    #[cfg(feature = "physics")]
    pub gravity: glam::Vec2,
    /// Number of physics substeps per game update. Default: 1.
    /// For high-precision physics (e.g., pool/billiards), use 4 for 240Hz physics
    /// with 60Hz game updates. Physics dt = fixed_dt / physics_substeps.
    #[cfg(feature = "physics")]
    pub physics_substeps: u32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            fixed_dt: 1.0 / 60.0,
            world_width: 800.0,
            world_height: 600.0,
            // Increased capacities for large-scale games (tilemaps, many units)
            max_entities: 2048,
            max_instances: 2048,
            max_effects_vertices: 16384,
            max_sounds: 32,
            max_events: 32,
            max_sdf_instances: 256,
            #[cfg(feature = "vectors")]
            max_vector_vertices: 16384,
            max_layer_batches: DEFAULT_MAX_LAYER_BATCHES,
            max_lights: DEFAULT_MAX_LIGHTS,
            effects_seed: 42,
            #[cfg(feature = "physics")]
            gravity: glam::Vec2::ZERO,
            #[cfg(feature = "physics")]
            physics_substeps: 1,
        }
    }
}

// ============================================================================
// Game Trait
// ============================================================================

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

// ============================================================================
// BakeState
// ============================================================================

/// Manages layer baking state for render caching.
///
/// Layers can be "baked" to an intermediate texture and reused across frames
/// for static content (e.g., terrain). The generation counter tracks changes
/// so the renderer knows when to refresh cached textures.
#[derive(Debug, Clone, Default)]
pub struct BakeState {
    /// Bitmask of layers marked for baking (bits 0-5 correspond to RenderLayer variants).
    mask: u8,
    /// Monotonic counter incremented on every bake/invalidate call.
    generation: u32,
}

impl BakeState {
    /// Create a new BakeState with no baked layers.
    pub fn new() -> Self {
        Self { mask: 0, generation: 0 }
    }

    /// Mark a layer for baking. The renderer will cache this layer's contents
    /// to an intermediate texture and reuse it until `invalidate()` is called.
    pub fn bake(&mut self, layer: RenderLayer) {
        self.mask |= 1 << layer.as_u8();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Mark a baked layer as dirty, signaling the renderer to re-render
    /// this layer's cached texture on the next frame.
    pub fn invalidate(&mut self, _layer: RenderLayer) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Remove a layer from baking — it will be rendered live every frame.
    pub fn unbake(&mut self, layer: RenderLayer) {
        self.mask &= !(1 << layer.as_u8());
        self.generation = self.generation.wrapping_add(1);
    }

    /// Get the baked layers bitmask (bits 0-5 correspond to RenderLayer variants).
    pub fn mask(&self) -> u8 {
        self.mask
    }

    /// Get the bake generation counter (monotonically increasing).
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Encode bake state as a single f32 for the SAB header.
    /// Format: `baked_mask | (bake_generation << 6)` stored as f32.
    /// f32 can represent integers up to 2^24 exactly, giving ~262k generations.
    pub fn encode(&self) -> f32 {
        let encoded = (self.mask as u32) | (self.generation << 6);
        encoded as f32
    }
}

// ============================================================================
// EngineContext
// ============================================================================

/// Mutable access to engine state, passed to Game::init and Game::update.
pub struct EngineContext {
    // -- Core state --
    pub scene: Scene,
    pub effects: EffectsState,
    pub sounds: Vec<SoundEvent>,
    pub events: Vec<GameEvent>,

    // -- Rendering state --
    /// Camera for 2D projection. Games can modify for pan/zoom.
    pub camera: Camera2D,
    /// Dynamic lighting state — persistent lights and ambient color.
    pub lights: LightState,
    /// Layer baking state for render caching.
    pub bake: BakeState,

    // -- Optional systems --
    #[cfg(feature = "vectors")]
    pub vectors: VectorState,
    #[cfg(feature = "physics")]
    pub physics: PhysicsWorld,

    // -- Private state --
    next_id: u32,
    sprite_registry: SpriteRegistry,
    #[cfg(feature = "physics")]
    collision_events: Vec<CollisionPair>,
}

// -- Constructors --

impl EngineContext {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            effects: EffectsState::new(42),
            sounds: Vec::new(),
            events: Vec::new(),
            camera: Camera2D::new(800.0, 600.0),
            lights: LightState::new(),
            bake: BakeState::new(),
            next_id: 1,
            sprite_registry: SpriteRegistry::new(),
            #[cfg(feature = "vectors")]
            vectors: VectorState::new(),
            #[cfg(feature = "physics")]
            physics: PhysicsWorld::new(Vec2::ZERO),
            #[cfg(feature = "physics")]
            collision_events: Vec::new(),
        }
    }

    /// Create an EngineContext configured from a GameConfig.
    /// This wires capacity settings to all subsystems.
    pub fn with_config(config: &GameConfig) -> Self {
        Self {
            scene: Scene::with_capacity(config.max_entities),
            effects: EffectsState::with_capacity(config.effects_seed, config.max_effects_vertices),
            sounds: Vec::with_capacity(config.max_sounds),
            events: Vec::with_capacity(config.max_events),
            camera: Camera2D::new(config.world_width, config.world_height),
            lights: LightState::with_capacity(config.max_lights),
            bake: BakeState::new(),
            next_id: 1,
            sprite_registry: SpriteRegistry::new(),
            #[cfg(feature = "vectors")]
            vectors: VectorState::with_capacity(config.max_vector_vertices),
            #[cfg(feature = "physics")]
            physics: PhysicsWorld::new(config.gravity),
            #[cfg(feature = "physics")]
            collision_events: Vec::new(),
        }
    }

    /// Create an EngineContext with a custom gravity vector.
    #[cfg(feature = "physics")]
    pub fn with_gravity(gravity: Vec2) -> Self {
        Self {
            scene: Scene::new(),
            effects: EffectsState::new(42),
            sounds: Vec::new(),
            events: Vec::new(),
            camera: Camera2D::new(800.0, 600.0),
            lights: LightState::new(),
            bake: BakeState::new(),
            next_id: 1,
            sprite_registry: SpriteRegistry::new(),
            #[cfg(feature = "vectors")]
            vectors: VectorState::new(),
            physics: PhysicsWorld::new(gravity),
            collision_events: Vec::new(),
        }
    }
}

// -- Core methods --

impl EngineContext {
    /// Generate the next unique entity ID.
    pub fn next_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Load an asset manifest (JSON) and populate the sprite registry.
    /// Can be called multiple times — each call replaces the registry.
    pub fn load_manifest(&mut self, json: &str) -> Result<(), String> {
        let manifest = AssetManifest::from_json(json).map_err(|e| e.to_string())?;
        self.sprite_registry = SpriteRegistry::from_manifest(&manifest);
        Ok(())
    }

    /// Look up a named sprite from the asset manifest.
    /// Returns a clone of the SpriteComponent, or None if not found.
    pub fn sprite(&self, name: &str) -> Option<SpriteComponent> {
        self.sprite_registry.get(name).cloned()
    }

    /// Emit a sound event to be forwarded to TypeScript.
    pub fn emit_sound(&mut self, event: SoundEvent) {
        self.sounds.push(event);
    }

    /// Emit a game event to be forwarded to TypeScript.
    pub fn emit_event(&mut self, event: GameEvent) {
        self.events.push(event);
    }

    /// Clear per-frame transient data (sounds, events, collision events, vectors).
    pub fn clear_frame_data(&mut self) {
        self.sounds.clear();
        self.events.clear();
        #[cfg(feature = "vectors")]
        self.vectors.clear();
        #[cfg(feature = "physics")]
        self.collision_events.clear();
    }
}

// -- Layer baking methods (delegate to BakeState) --

impl EngineContext {
    /// Mark a layer for baking. The renderer will cache this layer's contents
    /// to an intermediate texture and reuse it until `invalidate_layer()` is called.
    ///
    /// Call once during `init()` for static layers (e.g., terrain).
    /// After modifying entities on a baked layer, call `invalidate_layer()` then
    /// `bake_layer()` again to refresh the cache.
    pub fn bake_layer(&mut self, layer: RenderLayer) {
        self.bake.bake(layer);
    }

    /// Mark a baked layer as dirty, signaling the renderer to re-render
    /// this layer's cached texture on the next frame.
    pub fn invalidate_layer(&mut self, layer: RenderLayer) {
        self.bake.invalidate(layer);
    }

    /// Remove a layer from baking — it will be rendered live every frame.
    pub fn unbake_layer(&mut self, layer: RenderLayer) {
        self.bake.unbake(layer);
    }

    /// Get the baked layers bitmask (bits 0-5 correspond to RenderLayer variants).
    pub fn baked_layers_mask(&self) -> u8 {
        self.bake.mask()
    }

    /// Get the bake generation counter (monotonically increasing).
    pub fn bake_generation(&self) -> u32 {
        self.bake.generation()
    }

    /// Encode bake state as a single f32 for the SAB header.
    /// Format: `baked_mask | (bake_generation << 6)` stored as f32.
    /// f32 can represent integers up to 2^24 exactly, giving ~262k generations.
    pub fn bake_state_encoded(&self) -> f32 {
        self.bake.encode()
    }
}

// -- Text convenience methods --

impl EngineContext {
    /// Spawn text as a series of character entities.
    ///
    /// Each printable character becomes an Entity with a SpriteComponent.
    /// Characters outside the font's range are skipped.
    /// All character entities share the given `tag` for batch despawn.
    ///
    /// Returns the EntityIds of all spawned characters.
    pub fn spawn_text(
        &mut self,
        text: &str,
        pos: Vec2,
        size: f32,
        font: &FontConfig,
        tag: &str,
    ) -> Vec<EntityId> {
        // Use borrow-split pattern to avoid conflict between next_id() and scene.spawn()
        let mut next = self.next_id;
        let entities = build_text_entities(text, pos, size, font, tag, &mut || {
            let id = EntityId(next);
            next += 1;
            id
        });
        self.next_id = next;

        let ids: Vec<EntityId> = entities.iter().map(|e| e.id).collect();
        for entity in entities {
            self.scene.spawn(entity);
        }
        ids
    }

    /// Despawn all entities with the given tag.
    ///
    /// Useful for removing text that was spawned with a shared tag.
    pub fn despawn_text(&mut self, tag: &str) {
        despawn_text(&mut self.scene, tag);
    }
}

// -- Physics convenience methods --
// NOTE: ISP compliance via feature-gating.
// Games that don't enable `physics` feature won't see these methods at all.
// This prevents forcing games to "depend" on physics API they don't use.

#[cfg(feature = "physics")]
impl EngineContext {
    /// Spawn an entity with a physics body. Returns the EntityId.
    /// The entity's position is set from the BodyDesc.
    pub fn spawn_with_body(
        &mut self,
        entity: Entity,
        desc: BodyDesc,
        material: ColliderMaterial,
    ) -> EntityId {
        let id = entity.id;
        let body = self.physics.create_body(id, &desc, material);
        let entity = entity.with_body(body);
        self.scene.spawn(entity);
        id
    }

    /// Despawn an entity, cleaning up its physics body if present.
    pub fn despawn(&mut self, id: EntityId) {
        if let Some(entity) = self.scene.despawn(id) {
            if let Some(body) = &entity.body {
                self.physics.remove_body(body);
            }
        }
    }

    /// Apply a continuous force to an entity's physics body.
    pub fn apply_force(&mut self, id: EntityId, force: Vec2) {
        if let Some(entity) = self.scene.get(id) {
            if let Some(body) = &entity.body {
                self.physics.apply_force(body, force);
            }
        }
    }

    /// Apply an instantaneous impulse to an entity's physics body.
    pub fn apply_impulse(&mut self, id: EntityId, impulse: Vec2) {
        if let Some(entity) = self.scene.get(id) {
            if let Some(body) = &entity.body {
                self.physics.apply_impulse(body, impulse);
            }
        }
    }

    /// Set the linear velocity of an entity's physics body.
    pub fn set_velocity(&mut self, id: EntityId, vel: Vec2) {
        if let Some(entity) = self.scene.get(id) {
            if let Some(body) = &entity.body {
                self.physics.set_velocity(body, vel);
            }
        }
    }

    /// Get the linear velocity of an entity's physics body.
    pub fn velocity(&self, id: EntityId) -> Vec2 {
        self.scene
            .get(id)
            .and_then(|e| e.body.as_ref())
            .map(|body| self.physics.velocity(body))
            .unwrap_or(Vec2::ZERO)
    }

    /// Create a joint between two entities' physics bodies.
    /// Returns None if either entity lacks a physics body.
    pub fn create_joint(
        &mut self,
        entity_a: EntityId,
        entity_b: EntityId,
        desc: &JointDesc,
    ) -> Option<JointHandle> {
        let body_a = self.scene.get(entity_a)?.body.as_ref()?.clone();
        let body_b = self.scene.get(entity_b)?.body.as_ref()?.clone();
        Some(self.physics.create_joint(&body_a, &body_b, desc))
    }

    /// Remove a joint from the simulation.
    pub fn remove_joint(&mut self, handle: JointHandle) {
        self.physics.remove_joint(handle);
    }

    /// Get collision events from the most recent physics step.
    pub fn collisions(&self) -> &[CollisionPair] {
        &self.collision_events
    }

    /// Step the physics simulation and sync positions back to entities.
    /// Called automatically by the game runner after `Game::update()`.
    pub fn step_physics(&mut self) {
        self.collision_events.clear();
        self.physics.step_into(&mut self.collision_events);

        // Sync Rapier body positions back to entity positions
        for entity in self.scene.iter_mut() {
            if let Some(body) = &entity.body {
                let (pos, rot) = self.physics.body_position(body);
                entity.pos = pos;
                entity.rotation = rot;
            }
        }
    }
}

impl Default for EngineContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RenderContext
// ============================================================================

/// Render context for optional custom render commands.
pub struct RenderContext<'a> {
    pub render_buffer: &'a mut RenderBuffer,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod sprite_registry_tests {
    use super::*;
    use crate::components::sprite::AtlasId;

    #[test]
    fn sprite_after_load_manifest() {
        let mut ctx = EngineContext::new();
        let json = r#"{
            "atlases": [
                { "name": "tiles", "cols": 16, "rows": 8, "path": "tiles.png" }
            ],
            "sprites": {
                "hero": { "atlas": 0, "col": 3, "row": 5, "span": 2 }
            }
        }"#;

        ctx.load_manifest(json).unwrap();
        let hero = ctx.sprite("hero").expect("hero should exist");
        assert_eq!(hero.atlas, AtlasId(0));
        assert_eq!(hero.col, 3.0);
        assert_eq!(hero.row, 5.0);
        assert_eq!(hero.cell_span, 2.0);
        assert!(ctx.sprite("nonexistent").is_none());
    }
}

#[cfg(test)]
mod bake_state_tests {
    use super::*;

    #[test]
    fn bake_state_new_is_empty() {
        let bake = BakeState::new();
        assert_eq!(bake.mask(), 0);
        assert_eq!(bake.generation(), 0);
        assert_eq!(bake.encode(), 0.0);
    }

    #[test]
    fn bake_state_bake_sets_bit() {
        let mut bake = BakeState::new();
        bake.bake(RenderLayer::Terrain);
        assert_eq!(bake.mask(), 0b00_0010); // bit 1
        assert_eq!(bake.generation(), 1);
    }

    #[test]
    fn bake_state_unbake_clears_bit() {
        let mut bake = BakeState::new();
        bake.bake(RenderLayer::Background);
        bake.bake(RenderLayer::Terrain);
        assert_eq!(bake.mask(), 0b00_0011);

        bake.unbake(RenderLayer::Terrain);
        assert_eq!(bake.mask(), 0b00_0001); // only Background
    }

    #[test]
    fn bake_state_encode_round_trip() {
        let mut bake = BakeState::new();
        bake.bake(RenderLayer::Terrain);   // mask = 0b10, gen = 1
        bake.bake(RenderLayer::Foreground); // mask = 0b1010, gen = 2

        let encoded = bake.encode();
        let raw = encoded as u32;
        let decoded_mask = raw & 0x3F;
        let decoded_gen = raw >> 6;

        assert_eq!(decoded_mask, 0b00_1010); // Terrain(1) + Foreground(3)
        assert_eq!(decoded_gen, 2);
    }
}

#[cfg(test)]
mod bake_tests {
    use super::*;

    #[test]
    fn initial_bake_state_is_empty() {
        let ctx = EngineContext::new();
        assert_eq!(ctx.baked_layers_mask(), 0);
        assert_eq!(ctx.bake_generation(), 0);
        assert_eq!(ctx.bake_state_encoded(), 0.0);
    }

    #[test]
    fn bake_layer_sets_bit_and_increments_gen() {
        let mut ctx = EngineContext::new();
        ctx.bake_layer(RenderLayer::Terrain);

        assert_eq!(ctx.baked_layers_mask(), 0b00_0010); // bit 1
        assert_eq!(ctx.bake_generation(), 1);
    }

    #[test]
    fn bake_multiple_layers() {
        let mut ctx = EngineContext::new();
        ctx.bake_layer(RenderLayer::Background);
        ctx.bake_layer(RenderLayer::Terrain);

        assert_eq!(ctx.baked_layers_mask(), 0b00_0011); // bits 0 and 1
        assert_eq!(ctx.bake_generation(), 2);
    }

    #[test]
    fn invalidate_increments_gen_without_changing_mask() {
        let mut ctx = EngineContext::new();
        ctx.bake_layer(RenderLayer::Terrain);
        let mask_before = ctx.baked_layers_mask();
        let gen_before = ctx.bake_generation();

        ctx.invalidate_layer(RenderLayer::Terrain);

        assert_eq!(ctx.baked_layers_mask(), mask_before);
        assert_eq!(ctx.bake_generation(), gen_before + 1);
    }

    #[test]
    fn unbake_layer_clears_bit() {
        let mut ctx = EngineContext::new();
        ctx.bake_layer(RenderLayer::Terrain);
        ctx.bake_layer(RenderLayer::Background);
        assert_eq!(ctx.baked_layers_mask(), 0b00_0011);

        ctx.unbake_layer(RenderLayer::Terrain);
        assert_eq!(ctx.baked_layers_mask(), 0b00_0001); // only Background
    }

    #[test]
    fn bake_state_encoding_round_trip() {
        let mut ctx = EngineContext::new();
        ctx.bake_layer(RenderLayer::Terrain);   // mask = 0b10, gen = 1
        ctx.bake_layer(RenderLayer::Foreground); // mask = 0b1010, gen = 2

        let encoded = ctx.bake_state_encoded();
        let raw = encoded as u32;
        let decoded_mask = raw & 0x3F;
        let decoded_gen = raw >> 6;

        assert_eq!(decoded_mask, 0b00_1010); // Terrain(1) + Foreground(3)
        assert_eq!(decoded_gen, 2);
    }
}

#[cfg(test)]
mod camera_tests {
    use super::*;

    #[test]
    fn with_config_initializes_camera() {
        let mut config = GameConfig::default();
        config.world_width = 1920.0;
        config.world_height = 1080.0;

        let ctx = EngineContext::with_config(&config);
        assert_eq!(ctx.camera.width, 1920.0);
        assert_eq!(ctx.camera.height, 1080.0);
    }
}

#[cfg(test)]
#[cfg(feature = "physics")]
mod physics_tests {
    use super::*;
    use crate::core::physics::{BodyDesc, ColliderDesc, ColliderMaterial};

    #[test]
    fn spawn_with_body_creates_entity_and_physics() {
        let mut ctx = EngineContext::new();
        let id = ctx.next_id();
        let entity = Entity::new(id).with_pos(Vec2::new(100.0, 200.0));
        let desc = BodyDesc::dynamic(ColliderDesc::Ball { radius: 10.0 })
            .with_position(Vec2::new(100.0, 200.0));

        ctx.spawn_with_body(entity, desc, ColliderMaterial::default());

        assert_eq!(ctx.scene.len(), 1);
        assert_eq!(ctx.physics.body_count(), 1);
        assert!(ctx.scene.get(id).unwrap().body.is_some());
    }

    #[test]
    fn despawn_cleans_up_physics() {
        let mut ctx = EngineContext::new();
        let id = ctx.next_id();
        let entity = Entity::new(id);
        let desc = BodyDesc::dynamic(ColliderDesc::Ball { radius: 10.0 });

        ctx.spawn_with_body(entity, desc, ColliderMaterial::default());
        assert_eq!(ctx.physics.body_count(), 1);

        ctx.despawn(id);
        assert_eq!(ctx.scene.len(), 0);
        assert_eq!(ctx.physics.body_count(), 0);
    }

    #[test]
    fn step_physics_syncs_positions() {
        let mut ctx = EngineContext::with_gravity(Vec2::new(0.0, 100.0));
        ctx.physics.set_dt(1.0 / 60.0);

        let id = ctx.next_id();
        let entity = Entity::new(id).with_pos(Vec2::new(100.0, 0.0));
        let desc = BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
            .with_position(Vec2::new(100.0, 0.0));

        ctx.spawn_with_body(entity, desc, ColliderMaterial::default());

        // Step a few times
        for _ in 0..10 {
            ctx.step_physics();
        }

        let entity = ctx.scene.get(id).unwrap();
        // Entity position should have been synced from physics (moved downward)
        assert!(
            entity.pos.y > 0.0,
            "Entity should have moved down: y={}",
            entity.pos.y
        );
    }
}
