pub mod api;
pub mod core;
pub mod components;
pub mod systems;
pub mod renderer;
pub mod bridge;
pub mod input;
pub mod assets;
pub mod extensions;

// Re-export key types at crate root for convenience
pub use api::game::{Game, GameConfig, EngineContext, RenderContext, BakeState};
pub use api::types::{EntityId, SoundEvent, GameEvent};
pub use components::entity::Entity;
pub use components::layer::RenderLayer;
pub use components::sprite::{SpriteComponent, AtlasId, BlendMode};
pub use core::scene::Scene;
pub use core::time::FixedTimestep;
pub use renderer::instance::{RenderInstance, RenderBuffer};
pub use renderer::camera::Camera2D;
pub use input::queue::{InputEvent, InputQueue};
pub use assets::manifest::AssetManifest;
pub use assets::registry::SpriteRegistry;
pub use bridge::protocol::ProtocolLayout;
pub use systems::effects::{EffectsState, ElectricArc, Particle, SegmentColor, DebugLine};
pub use systems::render::LayerBatch;
pub use systems::text::FontConfig;
pub use systems::lighting::{PointLight, LightState};
pub use bridge::protocol::{LIGHT_FLOATS, DEFAULT_MAX_LIGHTS};
#[cfg(feature = "physics")]
pub use systems::debug::debug_draw_colliders;
pub use components::animation::{AnimationComponent, AnimationDef};
pub use components::emitter::{EmitterComponent, EmissionMode, ParticleColorMode};
pub use components::mesh::{MeshComponent, SDFShape, SDFColor};
pub use components::tilemap::{TilemapComponent, Tile};
pub use renderer::sdf_instance::{SDFInstance, SDFBuffer};
pub use systems::animation::tick_animations;

#[cfg(feature = "physics")]
pub use core::physics::{
    PhysicsWorld, PhysicsBody, BodyDesc, BodyType,
    ColliderDesc, ColliderMaterial, CollisionPair,
    JointHandle, JointDesc,
};

#[cfg(feature = "vectors")]
pub use systems::vector::{VectorState, VectorVertex, VectorColor};

// Extensions — decoupled optional systems
pub use extensions::{
    Easing, lerp, lerp_vec2, ease, ease_vec2,
    TransformGraph, LocalTransform,
    TweenState, Tween, TweenId, TweenTarget, TweenLoop,
};
