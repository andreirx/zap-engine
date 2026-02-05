use crate::core::scene::Scene;
use crate::api::types::{EntityId, SoundEvent, GameEvent};
use crate::input::queue::InputQueue;
use crate::renderer::instance::RenderBuffer;
use crate::systems::effects::EffectsState;
#[cfg(feature = "physics")]
use crate::core::physics::{
    PhysicsWorld, BodyDesc, ColliderMaterial, CollisionPair,
};
#[cfg(feature = "physics")]
use crate::components::entity::Entity;
#[cfg(feature = "physics")]
use glam::Vec2;

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
    /// Gravity vector for physics simulation. Default: zero (no gravity).
    /// For Y-down coordinate systems, use positive Y for downward gravity.
    #[cfg(feature = "physics")]
    pub gravity: glam::Vec2,
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
            #[cfg(feature = "physics")]
            gravity: glam::Vec2::ZERO,
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
    #[cfg(feature = "physics")]
    pub physics: PhysicsWorld,
    #[cfg(feature = "physics")]
    collision_events: Vec<CollisionPair>,
}

impl EngineContext {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            effects: EffectsState::new(42),
            sounds: Vec::new(),
            events: Vec::new(),
            next_id: 1,
            #[cfg(feature = "physics")]
            physics: PhysicsWorld::new(Vec2::ZERO),
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
            next_id: 1,
            physics: PhysicsWorld::new(gravity),
            collision_events: Vec::new(),
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

    /// Clear per-frame transient data (sounds, events, collision events).
    pub fn clear_frame_data(&mut self) {
        self.sounds.clear();
        self.events.clear();
        #[cfg(feature = "physics")]
        self.collision_events.clear();
    }

    // -- Physics convenience methods --

    /// Spawn an entity with a physics body. Returns the EntityId.
    /// The entity's position is set from the BodyDesc.
    #[cfg(feature = "physics")]
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
    #[cfg(feature = "physics")]
    pub fn despawn(&mut self, id: EntityId) {
        if let Some(entity) = self.scene.despawn(id) {
            if let Some(body) = &entity.body {
                self.physics.remove_body(body);
            }
        }
    }

    /// Apply a continuous force to an entity's physics body.
    #[cfg(feature = "physics")]
    pub fn apply_force(&mut self, id: EntityId, force: Vec2) {
        if let Some(entity) = self.scene.get(id) {
            if let Some(body) = &entity.body {
                self.physics.apply_force(body, force);
            }
        }
    }

    /// Apply an instantaneous impulse to an entity's physics body.
    #[cfg(feature = "physics")]
    pub fn apply_impulse(&mut self, id: EntityId, impulse: Vec2) {
        if let Some(entity) = self.scene.get(id) {
            if let Some(body) = &entity.body {
                self.physics.apply_impulse(body, impulse);
            }
        }
    }

    /// Set the linear velocity of an entity's physics body.
    #[cfg(feature = "physics")]
    pub fn set_velocity(&mut self, id: EntityId, vel: Vec2) {
        if let Some(entity) = self.scene.get(id) {
            if let Some(body) = &entity.body {
                self.physics.set_velocity(body, vel);
            }
        }
    }

    /// Get the linear velocity of an entity's physics body.
    #[cfg(feature = "physics")]
    pub fn velocity(&self, id: EntityId) -> Vec2 {
        self.scene
            .get(id)
            .and_then(|e| e.body.as_ref())
            .map(|body| self.physics.velocity(body))
            .unwrap_or(Vec2::ZERO)
    }

    /// Get collision events from the most recent physics step.
    #[cfg(feature = "physics")]
    pub fn collisions(&self) -> &[CollisionPair] {
        &self.collision_events
    }

    /// Step the physics simulation and sync positions back to entities.
    /// Called automatically by the game runner after `Game::update()`.
    #[cfg(feature = "physics")]
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

/// Render context for optional custom render commands.
pub struct RenderContext<'a> {
    pub render_buffer: &'a mut RenderBuffer,
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
