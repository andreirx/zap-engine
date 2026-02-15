use glam::Vec2;
use crate::api::types::EntityId;
use crate::components::animation::AnimationComponent;
use crate::components::layer::RenderLayer;
use crate::components::sprite::SpriteComponent;
use crate::components::emitter::EmitterComponent;
use crate::components::mesh::MeshComponent;
#[cfg(feature = "physics")]
use crate::core::physics::PhysicsBody;

/// Fat Entity — a single struct with optional components.
/// Designed for simplicity and rapid prototyping over ECS purity.
#[derive(Debug, Clone)]
pub struct Entity {
    /// Unique identifier.
    pub id: EntityId,
    /// String tag for finding entities by name.
    pub tag: String,
    /// Whether this entity is active (inactive entities are skipped).
    pub active: bool,
    /// Render layer — controls draw order (Background..UI). Default: Objects.
    pub layer: RenderLayer,
    /// Position in world space.
    pub pos: Vec2,
    /// Rotation in radians.
    pub rotation: f32,
    /// Scale (world-space size). For sprites, this is the rendered size in world units.
    pub scale: Vec2,
    /// Sprite component (optional — entities without sprites are invisible).
    pub sprite: Option<SpriteComponent>,
    /// Physics body (optional — requires "physics" feature).
    #[cfg(feature = "physics")]
    pub body: Option<PhysicsBody>,
    /// Particle emitter (optional — auto-spawns particles at entity position).
    pub emitter: Option<EmitterComponent>,
    /// SDF mesh (optional — rendered via the molecule pipeline).
    pub mesh: Option<MeshComponent>,
    /// Animation (optional — auto-updates sprite col/row each frame).
    pub animation: Option<AnimationComponent>,
}

impl Entity {
    /// Create a new entity with the given ID at the origin.
    pub fn new(id: EntityId) -> Self {
        Self {
            id,
            tag: String::new(),
            active: true,
            layer: RenderLayer::default(),
            pos: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            sprite: None,
            #[cfg(feature = "physics")]
            body: None,
            emitter: None,
            mesh: None,
            animation: None,
        }
    }

    // -- Builder pattern --

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    pub fn with_layer(mut self, layer: RenderLayer) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_pos(mut self, pos: Vec2) -> Self {
        self.pos = pos;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_sprite(mut self, sprite: SpriteComponent) -> Self {
        self.sprite = Some(sprite);
        self
    }

    #[cfg(feature = "physics")]
    pub fn with_body(mut self, body: PhysicsBody) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_emitter(mut self, emitter: EmitterComponent) -> Self {
        self.emitter = Some(emitter);
        self
    }

    pub fn with_mesh(mut self, mesh: MeshComponent) -> Self {
        self.mesh = Some(mesh);
        self
    }

    pub fn with_animation(mut self, animation: AnimationComponent) -> Self {
        self.animation = Some(animation);
        self
    }
}
