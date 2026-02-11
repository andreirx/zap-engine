use glam::Vec2;
use rapier2d::prelude::*;
use std::sync::Mutex;

use crate::api::types::EntityId;

// ---------------------------------------------------------------------------
// Conversion helpers (private) — glam ↔ nalgebra
// ---------------------------------------------------------------------------

fn vec2_to_na(v: Vec2) -> nalgebra::Vector2<f32> {
    nalgebra::Vector2::new(v.x, v.y)
}

fn na_to_vec2(v: &nalgebra::Vector2<f32>) -> Vec2 {
    Vec2::new(v.x, v.y)
}

fn na_iso_to_pos_rot(iso: &nalgebra::Isometry2<f32>) -> (Vec2, f32) {
    let pos = Vec2::new(iso.translation.x, iso.translation.y);
    let rot = iso.rotation.angle();
    (pos, rot)
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The kind of rigid body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyType {
    Dynamic,
    Fixed,
    KinematicPositionBased,
    KinematicVelocityBased,
}

impl BodyType {
    fn to_rapier(self) -> RigidBodyType {
        match self {
            BodyType::Dynamic => RigidBodyType::Dynamic,
            BodyType::Fixed => RigidBodyType::Fixed,
            BodyType::KinematicPositionBased => RigidBodyType::KinematicPositionBased,
            BodyType::KinematicVelocityBased => RigidBodyType::KinematicVelocityBased,
        }
    }
}

/// Shape description for a collider.
#[derive(Debug, Clone, Copy)]
pub enum ColliderDesc {
    Ball { radius: f32 },
    Cuboid { half_width: f32, half_height: f32 },
    CapsuleY { half_height: f32, radius: f32 },
}

impl ColliderDesc {
    fn build_collider(&self) -> ColliderBuilder {
        match *self {
            ColliderDesc::Ball { radius } => ColliderBuilder::ball(radius),
            ColliderDesc::Cuboid { half_width, half_height } => {
                ColliderBuilder::cuboid(half_width, half_height)
            }
            ColliderDesc::CapsuleY { half_height, radius } => {
                ColliderBuilder::capsule_y(half_height, radius)
            }
        }
    }
}

/// Physical material properties for a collider.
#[derive(Debug, Clone, Copy)]
pub struct ColliderMaterial {
    pub restitution: f32,
    pub friction: f32,
    pub density: f32,
}

impl Default for ColliderMaterial {
    fn default() -> Self {
        Self {
            restitution: 0.3,
            friction: 0.5,
            density: 1.0,
        }
    }
}

/// Builder for describing a rigid body before creation.
#[derive(Debug, Clone)]
pub struct BodyDesc {
    pub body_type: BodyType,
    pub position: Vec2,
    pub rotation: f32,
    pub velocity: Vec2,
    pub gravity_scale: f32,
    pub fixed_rotation: bool,
    pub ccd: bool,
    pub collider: ColliderDesc,
    pub linear_damping: f32,
    pub angular_damping: f32,
}

impl BodyDesc {
    /// Create a dynamic body description with the given collider shape.
    pub fn dynamic(collider: ColliderDesc) -> Self {
        Self {
            body_type: BodyType::Dynamic,
            position: Vec2::ZERO,
            rotation: 0.0,
            velocity: Vec2::ZERO,
            gravity_scale: 1.0,
            fixed_rotation: false,
            ccd: false,
            collider,
            linear_damping: 0.0,
            angular_damping: 0.0,
        }
    }

    /// Create a fixed (static) body description with the given collider shape.
    pub fn fixed(collider: ColliderDesc) -> Self {
        Self {
            body_type: BodyType::Fixed,
            position: Vec2::ZERO,
            rotation: 0.0,
            velocity: Vec2::ZERO,
            gravity_scale: 0.0,
            fixed_rotation: true,
            ccd: false,
            collider,
            linear_damping: 0.0,
            angular_damping: 0.0,
        }
    }

    pub fn with_position(mut self, pos: Vec2) -> Self {
        self.position = pos;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_velocity(mut self, vel: Vec2) -> Self {
        self.velocity = vel;
        self
    }

    pub fn with_gravity_scale(mut self, scale: f32) -> Self {
        self.gravity_scale = scale;
        self
    }

    pub fn with_fixed_rotation(mut self, fixed: bool) -> Self {
        self.fixed_rotation = fixed;
        self
    }

    pub fn with_ccd(mut self, enabled: bool) -> Self {
        self.ccd = enabled;
        self
    }

    /// Set the linear damping (velocity decay). Higher values slow the body faster.
    /// Useful for simulating friction on surfaces like pool table felt.
    pub fn with_linear_damping(mut self, damping: f32) -> Self {
        self.linear_damping = damping;
        self
    }

    /// Set the angular damping (rotation decay). Higher values slow rotation faster.
    pub fn with_angular_damping(mut self, damping: f32) -> Self {
        self.angular_damping = damping;
        self
    }
}

/// Handle pair stored on an Entity, referencing Rapier internals.
#[derive(Debug, Clone, Copy)]
pub struct PhysicsBody {
    pub body_handle: RigidBodyHandle,
    pub collider_handle: ColliderHandle,
}

/// Handle to a joint in the physics simulation.
#[derive(Debug, Clone, Copy)]
pub struct JointHandle(pub(crate) ImpulseJointHandle);

/// Description of a joint to create between two bodies.
#[derive(Debug, Clone, Copy)]
pub enum JointDesc {
    /// Rigidly locks two bodies together at the given local anchors.
    Fixed { anchor_a: Vec2, anchor_b: Vec2 },
    /// Spring/distance joint that applies forces to maintain rest length.
    Spring {
        anchor_a: Vec2,
        anchor_b: Vec2,
        rest_length: f32,
        stiffness: f32,
        damping: f32,
    },
    /// Allows free rotation around the anchor points (hinge joint in 2D).
    Revolute { anchor_a: Vec2, anchor_b: Vec2 },
}

/// A collision event between two entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionPair {
    pub entity_a: EntityId,
    pub entity_b: EntityId,
    /// `true` when the collision just started, `false` when it ended.
    pub started: bool,
}

// ---------------------------------------------------------------------------
// WASM-safe event collector (no crossbeam)
// ---------------------------------------------------------------------------

struct DirectEventCollector {
    collisions: Mutex<Vec<CollisionEvent>>,
}

impl DirectEventCollector {
    fn new() -> Self {
        Self {
            collisions: Mutex::new(Vec::new()),
        }
    }

    fn drain_collisions(&self) -> Vec<CollisionEvent> {
        std::mem::take(&mut *self.collisions.lock().unwrap())
    }
}

impl EventHandler for DirectEventCollector {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: CollisionEvent,
        _contact_pair: Option<&ContactPair>,
    ) {
        self.collisions.lock().unwrap().push(event);
    }

    fn handle_contact_force_event(
        &self,
        _dt: f32,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _contact_pair: &ContactPair,
        _total_force_magnitude: f32,
    ) {
        // We don't use contact force events yet but the trait requires this.
    }
}

// ---------------------------------------------------------------------------
// PhysicsWorld
// ---------------------------------------------------------------------------

/// Wraps all Rapier2D boilerplate into a single, easy-to-use struct.
pub struct PhysicsWorld {
    gravity: nalgebra::Vector2<f32>,
    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    pub(crate) bodies: RigidBodySet,
    pub(crate) colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,
    event_collector: DirectEventCollector,
}

impl PhysicsWorld {
    /// Create a new physics world with the given gravity vector.
    /// For Y-down coordinate systems, use positive Y for downward gravity
    /// (e.g., `Vec2::new(0.0, 981.0)` for ~10× Earth gravity in pixels).
    pub fn new(gravity: Vec2) -> Self {
        Self {
            gravity: vec2_to_na(gravity),
            integration_parameters: IntegrationParameters::default(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            event_collector: DirectEventCollector::new(),
        }
    }

    /// Set the integration timestep.
    pub fn set_dt(&mut self, dt: f32) {
        self.integration_parameters.dt = dt;
    }

    /// Create a rigid body + collider and return handles.
    /// The EntityId is stored in the body's `user_data` for collision lookups.
    pub fn create_body(
        &mut self,
        entity_id: EntityId,
        desc: &BodyDesc,
        material: ColliderMaterial,
    ) -> PhysicsBody {
        let rb = RigidBodyBuilder::new(desc.body_type.to_rapier())
            .translation(nalgebra::Vector2::new(desc.position.x, desc.position.y))
            .rotation(desc.rotation)
            .linvel(nalgebra::Vector2::new(desc.velocity.x, desc.velocity.y))
            .gravity_scale(desc.gravity_scale)
            .locked_axes(if desc.fixed_rotation {
                LockedAxes::ROTATION_LOCKED
            } else {
                LockedAxes::empty()
            })
            .ccd_enabled(desc.ccd)
            .linear_damping(desc.linear_damping)
            .angular_damping(desc.angular_damping)
            .user_data(entity_id.0 as u128)
            .build();

        let body_handle = self.bodies.insert(rb);

        let collider = desc
            .collider
            .build_collider()
            .restitution(material.restitution)
            .friction(material.friction)
            .density(material.density)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();

        let collider_handle =
            self.colliders
                .insert_with_parent(collider, body_handle, &mut self.bodies);

        PhysicsBody {
            body_handle,
            collider_handle,
        }
    }

    /// Remove a body and all its colliders from the simulation.
    pub fn remove_body(&mut self, body: &PhysicsBody) {
        self.bodies.remove(
            body.body_handle,
            &mut self.island_manager,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }

    /// Step the simulation and collect collision events into the provided Vec.
    pub fn step_into(&mut self, collision_events: &mut Vec<CollisionPair>) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &self.event_collector,
        );

        // Drain collision events and resolve entity IDs from user_data
        for event in self.event_collector.drain_collisions() {
            let (h1, h2, started) = match event {
                CollisionEvent::Started(h1, h2, _) => (h1, h2, true),
                CollisionEvent::Stopped(h1, h2, _) => (h1, h2, false),
            };

            // Resolve collider handles → body handles → entity IDs
            let entity_a = self.collider_to_entity(h1);
            let entity_b = self.collider_to_entity(h2);

            if let (Some(a), Some(b)) = (entity_a, entity_b) {
                collision_events.push(CollisionPair {
                    entity_a: a,
                    entity_b: b,
                    started,
                });
            }
        }
    }

    /// Apply a force to a body (continuous — call every frame).
    pub fn apply_force(&mut self, body: &PhysicsBody, force: Vec2) {
        if let Some(rb) = self.bodies.get_mut(body.body_handle) {
            rb.add_force(vec2_to_na(force), true);
        }
    }

    /// Apply an instantaneous impulse to a body.
    pub fn apply_impulse(&mut self, body: &PhysicsBody, impulse: Vec2) {
        if let Some(rb) = self.bodies.get_mut(body.body_handle) {
            rb.apply_impulse(vec2_to_na(impulse), true);
        }
    }

    /// Set the linear velocity of a body directly.
    pub fn set_velocity(&mut self, body: &PhysicsBody, vel: Vec2) {
        if let Some(rb) = self.bodies.get_mut(body.body_handle) {
            rb.set_linvel(vec2_to_na(vel), true);
        }
    }

    /// Get the current linear velocity of a body.
    pub fn velocity(&self, body: &PhysicsBody) -> Vec2 {
        self.bodies
            .get(body.body_handle)
            .map(|rb| na_to_vec2(rb.linvel()))
            .unwrap_or(Vec2::ZERO)
    }

    /// Set position and rotation for a kinematic body.
    pub fn set_kinematic_position(&mut self, body: &PhysicsBody, pos: Vec2, rotation: f32) {
        if let Some(rb) = self.bodies.get_mut(body.body_handle) {
            rb.set_next_kinematic_position(nalgebra::Isometry2::new(
                nalgebra::Vector2::new(pos.x, pos.y),
                rotation,
            ));
        }
    }

    /// Get the current position and rotation of a body.
    pub fn body_position(&self, body: &PhysicsBody) -> (Vec2, f32) {
        self.bodies
            .get(body.body_handle)
            .map(|rb| na_iso_to_pos_rot(rb.position()))
            .unwrap_or((Vec2::ZERO, 0.0))
    }

    /// Number of rigid bodies in the simulation.
    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    /// Query the collider shape of a physics body.
    /// Returns `None` if the collider no longer exists or has an unsupported shape.
    pub fn collider_shape(&self, body: &PhysicsBody) -> Option<ColliderDesc> {
        let collider = self.colliders.get(body.collider_handle)?;
        let shape = collider.shape();
        if let Some(ball) = shape.as_ball() {
            Some(ColliderDesc::Ball { radius: ball.radius })
        } else if let Some(cuboid) = shape.as_cuboid() {
            Some(ColliderDesc::Cuboid {
                half_width: cuboid.half_extents.x,
                half_height: cuboid.half_extents.y,
            })
        } else if let Some(capsule) = shape.as_capsule() {
            Some(ColliderDesc::CapsuleY {
                half_height: capsule.half_height(),
                radius: capsule.radius,
            })
        } else {
            None
        }
    }

    // -- Joint methods --

    /// Create a joint between two bodies. Returns a handle for later removal.
    pub fn create_joint(
        &mut self,
        body_a: &PhysicsBody,
        body_b: &PhysicsBody,
        desc: &JointDesc,
    ) -> JointHandle {
        let handle = match desc {
            JointDesc::Fixed { anchor_a, anchor_b } => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(nalgebra::Point2::new(anchor_a.x, anchor_a.y))
                    .local_anchor2(nalgebra::Point2::new(anchor_b.x, anchor_b.y))
                    .build();
                self.impulse_joints.insert(body_a.body_handle, body_b.body_handle, joint, true)
            }
            JointDesc::Spring { anchor_a, anchor_b, rest_length, stiffness, damping } => {
                let joint = SpringJointBuilder::new(*rest_length, *stiffness, *damping)
                    .local_anchor1(nalgebra::Point2::new(anchor_a.x, anchor_a.y))
                    .local_anchor2(nalgebra::Point2::new(anchor_b.x, anchor_b.y))
                    .build();
                self.impulse_joints.insert(body_a.body_handle, body_b.body_handle, joint, true)
            }
            JointDesc::Revolute { anchor_a, anchor_b } => {
                let joint = RevoluteJointBuilder::new()
                    .local_anchor1(nalgebra::Point2::new(anchor_a.x, anchor_a.y))
                    .local_anchor2(nalgebra::Point2::new(anchor_b.x, anchor_b.y))
                    .build();
                self.impulse_joints.insert(body_a.body_handle, body_b.body_handle, joint, true)
            }
        };
        JointHandle(handle)
    }

    /// Remove a joint from the simulation.
    pub fn remove_joint(&mut self, handle: JointHandle) {
        self.impulse_joints.remove(handle.0, true);
    }

    /// Number of joints in the simulation.
    pub fn joint_count(&self) -> usize {
        self.impulse_joints.len()
    }

    // -- private helpers --

    fn collider_to_entity(&self, collider_handle: ColliderHandle) -> Option<EntityId> {
        let collider = self.colliders.get(collider_handle)?;
        let body_handle = collider.parent()?;
        let body = self.bodies.get(body_handle)?;
        Some(EntityId(body.user_data as u32))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_remove_body() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        let body = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 10.0 }),
            ColliderMaterial::default(),
        );
        assert_eq!(world.body_count(), 1);
        world.remove_body(&body);
        assert_eq!(world.body_count(), 0);
    }

    #[test]
    fn gravity_affects_dynamic_body() {
        let mut world = PhysicsWorld::new(Vec2::new(0.0, 100.0));
        world.set_dt(1.0 / 60.0);

        let body = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 }),
            ColliderMaterial::default(),
        );

        let (initial_pos, _) = world.body_position(&body);
        let mut events = Vec::new();
        // Step a few times to let gravity take effect
        for _ in 0..10 {
            world.step_into(&mut events);
        }
        let (new_pos, _) = world.body_position(&body);

        // Body should have moved downward (positive Y = down)
        assert!(
            new_pos.y > initial_pos.y,
            "Body should fall: start={}, end={}",
            initial_pos.y,
            new_pos.y
        );
    }

    #[test]
    fn impulse_changes_velocity() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        let body = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 }),
            ColliderMaterial::default(),
        );

        assert_eq!(world.velocity(&body), Vec2::ZERO);
        world.apply_impulse(&body, Vec2::new(100.0, 0.0));

        // After impulse, velocity should be non-zero in X
        let mut events = Vec::new();
        world.step_into(&mut events);
        let vel = world.velocity(&body);
        assert!(vel.x > 0.0, "Velocity should be positive X: {:?}", vel);
    }

    #[test]
    fn set_velocity_directly() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        let body = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 }),
            ColliderMaterial::default(),
        );

        world.set_velocity(&body, Vec2::new(50.0, -30.0));
        let vel = world.velocity(&body);
        assert!((vel.x - 50.0).abs() < 0.001);
        assert!((vel.y - (-30.0)).abs() < 0.001);
    }

    #[test]
    fn fixed_body_does_not_move() {
        let mut world = PhysicsWorld::new(Vec2::new(0.0, 100.0));
        world.set_dt(1.0 / 60.0);

        let body = world.create_body(
            EntityId(1),
            &BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: 100.0,
                half_height: 10.0,
            })
            .with_position(Vec2::new(0.0, 500.0)),
            ColliderMaterial::default(),
        );

        let mut events = Vec::new();
        for _ in 0..10 {
            world.step_into(&mut events);
        }

        let (pos, _) = world.body_position(&body);
        assert!(
            (pos.y - 500.0).abs() < 0.001,
            "Fixed body should not move: y={}",
            pos.y
        );
    }

    #[test]
    fn collision_events_between_converging_bodies() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        world.set_dt(1.0 / 60.0);

        // Two balls moving toward each other
        let _body_a = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 10.0 })
                .with_position(Vec2::new(0.0, 0.0))
                .with_velocity(Vec2::new(200.0, 0.0)),
            ColliderMaterial::default(),
        );

        let _body_b = world.create_body(
            EntityId(2),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 10.0 })
                .with_position(Vec2::new(30.0, 0.0))
                .with_velocity(Vec2::new(-200.0, 0.0)),
            ColliderMaterial::default(),
        );

        let mut all_events = Vec::new();
        // Step many times — they should collide
        for _ in 0..60 {
            world.step_into(&mut all_events);
        }

        let started_events: Vec<_> = all_events.iter().filter(|e| e.started).collect();
        assert!(
            !started_events.is_empty(),
            "Should have at least one collision start event"
        );

        // Verify entity IDs are present (order may vary)
        let first = &started_events[0];
        let ids = [first.entity_a, first.entity_b];
        assert!(ids.contains(&EntityId(1)));
        assert!(ids.contains(&EntityId(2)));
    }

    #[test]
    fn builder_pattern() {
        let desc = BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
            .with_position(Vec2::new(10.0, 20.0))
            .with_velocity(Vec2::new(1.0, 2.0))
            .with_gravity_scale(0.5)
            .with_fixed_rotation(true)
            .with_ccd(true);

        assert_eq!(desc.body_type, BodyType::Dynamic);
        assert_eq!(desc.position, Vec2::new(10.0, 20.0));
        assert_eq!(desc.velocity, Vec2::new(1.0, 2.0));
        assert!((desc.gravity_scale - 0.5).abs() < 0.001);
        assert!(desc.fixed_rotation);
        assert!(desc.ccd);
    }

    #[test]
    fn collider_material_defaults() {
        let mat = ColliderMaterial::default();
        assert!((mat.restitution - 0.3).abs() < 0.001);
        assert!((mat.friction - 0.5).abs() < 0.001);
        assert!((mat.density - 1.0).abs() < 0.001);
    }

    #[test]
    fn body_position_and_rotation() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        let body = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(100.0, 200.0))
                .with_rotation(1.5),
            ColliderMaterial::default(),
        );

        let (pos, rot) = world.body_position(&body);
        assert!((pos.x - 100.0).abs() < 0.001);
        assert!((pos.y - 200.0).abs() < 0.001);
        assert!((rot - 1.5).abs() < 0.001);
    }

    #[test]
    fn collider_shape_ball_and_cuboid() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        let ball_body = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 15.0 }),
            ColliderMaterial::default(),
        );
        let cuboid_body = world.create_body(
            EntityId(2),
            &BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: 50.0,
                half_height: 10.0,
            }),
            ColliderMaterial::default(),
        );

        let ball_shape = world.collider_shape(&ball_body).expect("ball should have shape");
        match ball_shape {
            ColliderDesc::Ball { radius } => assert!((radius - 15.0).abs() < 0.001),
            _ => panic!("expected Ball, got {:?}", ball_shape),
        }

        let cuboid_shape = world.collider_shape(&cuboid_body).expect("cuboid should have shape");
        match cuboid_shape {
            ColliderDesc::Cuboid { half_width, half_height } => {
                assert!((half_width - 50.0).abs() < 0.001);
                assert!((half_height - 10.0).abs() < 0.001);
            }
            _ => panic!("expected Cuboid, got {:?}", cuboid_shape),
        }
    }

    #[test]
    fn create_and_remove_joint() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        let body_a = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(0.0, 0.0)),
            ColliderMaterial::default(),
        );
        let body_b = world.create_body(
            EntityId(2),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(50.0, 0.0)),
            ColliderMaterial::default(),
        );

        assert_eq!(world.joint_count(), 0);
        let handle = world.create_joint(&body_a, &body_b, &JointDesc::Fixed {
            anchor_a: Vec2::ZERO,
            anchor_b: Vec2::ZERO,
        });
        assert_eq!(world.joint_count(), 1);
        world.remove_joint(handle);
        assert_eq!(world.joint_count(), 0);
    }

    #[test]
    fn fixed_joint_constrains_bodies() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        world.set_dt(1.0 / 60.0);

        // Start both at same position so fixed joint is in rest configuration
        let body_a = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(0.0, 0.0)),
            ColliderMaterial::default(),
        );
        let body_b = world.create_body(
            EntityId(2),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(0.0, 0.0)),
            ColliderMaterial::default(),
        );

        world.create_joint(&body_a, &body_b, &JointDesc::Fixed {
            anchor_a: Vec2::ZERO,
            anchor_b: Vec2::ZERO,
        });

        // Apply a strong impulse to body A only
        world.apply_impulse(&body_a, Vec2::new(5000.0, 0.0));

        let mut events = Vec::new();
        for _ in 0..60 {
            world.step_into(&mut events);
        }

        // Both bodies should have moved right together
        let (pos_a, _) = world.body_position(&body_a);
        let (pos_b, _) = world.body_position(&body_b);
        assert!(pos_a.x > 1.0, "Body A should have moved right: x={}", pos_a.x);
        assert!(pos_b.x > 1.0, "Body B should be dragged along: x={}", pos_b.x);
        // They should stay close together (within a few units)
        assert!((pos_a.x - pos_b.x).abs() < 5.0,
            "Bodies should stay together: A.x={}, B.x={}", pos_a.x, pos_b.x);
    }

    #[test]
    fn spring_joint_applies_force() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        world.set_dt(1.0 / 60.0);

        let body_a = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(0.0, 0.0)),
            ColliderMaterial::default(),
        );
        let body_b = world.create_body(
            EntityId(2),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(100.0, 0.0)),
            ColliderMaterial::default(),
        );

        // Spring with rest_length=30, so the bodies are way beyond rest → spring pulls them
        world.create_joint(&body_a, &body_b, &JointDesc::Spring {
            anchor_a: Vec2::ZERO,
            anchor_b: Vec2::ZERO,
            rest_length: 30.0,
            stiffness: 500.0,
            damping: 5.0,
        });

        let mut events = Vec::new();
        for _ in 0..60 {
            world.step_into(&mut events);
        }

        // Bodies should have moved toward each other
        let (pos_a, _) = world.body_position(&body_a);
        let (pos_b, _) = world.body_position(&body_b);
        let distance = (pos_b.x - pos_a.x).abs();
        assert!(
            distance < 100.0,
            "Spring should pull bodies closer: distance={}",
            distance
        );
    }

    #[test]
    fn revolute_joint_allows_rotation() {
        let mut world = PhysicsWorld::new(Vec2::new(0.0, 500.0));
        world.set_dt(1.0 / 60.0);

        // Fixed pivot at (100, 100)
        let body_a = world.create_body(
            EntityId(1),
            &BodyDesc::fixed(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(100.0, 100.0)),
            ColliderMaterial::default(),
        );
        // Pendulum bob at (150, 100) — 50 units right of pivot
        let body_b = world.create_body(
            EntityId(2),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(150.0, 100.0)),
            ColliderMaterial::default(),
        );

        // Pivot in A's local space = (0,0), rod attachment in B's local space = (-50, 0)
        // World: A_anchor = (100,100) + (0,0) = (100,100)
        //        B_anchor = (150,100) + (-50,0) = (100,100) ✓ — matches
        world.create_joint(&body_a, &body_b, &JointDesc::Revolute {
            anchor_a: Vec2::ZERO,
            anchor_b: Vec2::new(-50.0, 0.0),
        });

        let mut events = Vec::new();
        for _ in 0..60 {
            world.step_into(&mut events);
        }

        // Gravity should swing body B downward (Y increases in Y-down coords)
        let (pos_b, _) = world.body_position(&body_b);
        assert!(pos_b.y > 105.0, "Body B should swing down: y={}", pos_b.y);
    }

    #[test]
    fn multiple_bodies_independent() {
        let mut world = PhysicsWorld::new(Vec2::ZERO);
        let body_a = world.create_body(
            EntityId(1),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(0.0, 0.0)),
            ColliderMaterial::default(),
        );
        let body_b = world.create_body(
            EntityId(2),
            &BodyDesc::dynamic(ColliderDesc::Ball { radius: 5.0 })
                .with_position(Vec2::new(1000.0, 1000.0)),
            ColliderMaterial::default(),
        );

        assert_eq!(world.body_count(), 2);

        world.set_velocity(&body_a, Vec2::new(10.0, 0.0));
        let vel_b = world.velocity(&body_b);
        assert!((vel_b.x).abs() < 0.001, "Body B should not be affected");

        world.remove_body(&body_a);
        assert_eq!(world.body_count(), 1);

        // Body B should still exist and be queryable
        let (pos_b, _) = world.body_position(&body_b);
        assert!((pos_b.x - 1000.0).abs() < 0.001);
    }
}
