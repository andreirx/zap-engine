use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::input::queue::{InputEvent, InputQueue};
use zap_engine::components::mesh::{MeshComponent, SDFColor};
use glam::Vec2;

use crate::elements::{self, element_data};
use crate::molecule::{MoleculeState, Bond};

const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;
const WALL_THICKNESS: f32 = 20.0;
const BOND_RADIUS: f32 = 3.0;
const BOND_COLOR: SDFColor = SDFColor { r: 0.5, g: 0.5, b: 0.5 };
const SPRING_STIFFNESS: f32 = 300.0;
const SPRING_DAMPING: f32 = 15.0;
const MAX_ATOMS: usize = 32;

/// Custom event kinds from React UI
const CUSTOM_SELECT_ELEMENT: u32 = 1;
const CUSTOM_CLEAR: u32 = 2;

/// Game event kinds to React
const EVENT_ATOM_COUNT: f32 = 1.0;
const EVENT_BOND_COUNT: f32 = 2.0;

pub struct ChemistryLab {
    selected_element: u32,
    molecule: MoleculeState,
    drag_from: Option<EntityId>,
    drag_active: bool,
}

impl ChemistryLab {
    pub fn new() -> Self {
        Self {
            selected_element: elements::CARBON,
            molecule: MoleculeState::new(),
            drag_from: None,
            drag_active: false,
        }
    }

    fn spawn_wall(ctx: &mut EngineContext, pos: Vec2, half_w: f32, half_h: f32) {
        let id = ctx.next_id();
        let entity = Entity::new(id).with_tag("wall");
        let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
            half_width: half_w,
            half_height: half_h,
        })
        .with_position(pos);
        ctx.spawn_with_body(entity, desc, ColliderMaterial::default());
    }

    fn spawn_atom(&mut self, ctx: &mut EngineContext, pos: Vec2) {
        if self.molecule.atoms.len() >= MAX_ATOMS {
            return;
        }

        let elem = match element_data(self.selected_element) {
            Some(e) => e,
            None => return,
        };

        let id = ctx.next_id();
        let entity = Entity::new(id)
            .with_tag("atom")
            .with_pos(pos)
            .with_mesh(MeshComponent::sphere(elem.radius, elem.color));

        let desc = BodyDesc::dynamic(ColliderDesc::Ball { radius: elem.radius })
            .with_position(pos);

        let material = ColliderMaterial {
            restitution: 0.3,
            friction: 0.3,
            density: 0.8,
        };

        ctx.spawn_with_body(entity, desc, material);
        self.molecule.add_atom(id, self.selected_element);
    }

    fn hit_test_atom(&self, ctx: &EngineContext, pos: Vec2) -> Option<EntityId> {
        for &(atom_id, kind, _) in &self.molecule.atoms {
            if let Some(entity) = ctx.scene.get(atom_id) {
                let elem = element_data(kind)?;
                let dist = entity.pos.distance(pos);
                if dist < elem.radius * 1.5 {
                    return Some(atom_id);
                }
            }
        }
        None
    }

    fn try_create_bond(&mut self, ctx: &mut EngineContext, atom_a: EntityId, atom_b: EntityId) {
        if atom_a == atom_b {
            return;
        }
        if self.molecule.has_bond_between(atom_a, atom_b) {
            return;
        }

        let kind_a = match self.molecule.element_kind(atom_a) {
            Some(k) => k,
            None => return,
        };
        let kind_b = match self.molecule.element_kind(atom_b) {
            Some(k) => k,
            None => return,
        };

        let elem_a = match element_data(kind_a) {
            Some(e) => e,
            None => return,
        };
        let elem_b = match element_data(kind_b) {
            Some(e) => e,
            None => return,
        };

        if !self.molecule.can_bond(atom_a, elem_a.max_bonds) {
            return;
        }
        if !self.molecule.can_bond(atom_b, elem_b.max_bonds) {
            return;
        }

        // Get atom positions for rest length
        let pos_a = match ctx.scene.get(atom_a) {
            Some(e) => e.pos,
            None => return,
        };
        let pos_b = match ctx.scene.get(atom_b) {
            Some(e) => e.pos,
            None => return,
        };

        let rest_length = pos_a.distance(pos_b);

        // Create spring joint
        let joint_handle = match ctx.create_joint(atom_a, atom_b, &JointDesc::Spring {
            anchor_a: Vec2::ZERO,
            anchor_b: Vec2::ZERO,
            rest_length,
            stiffness: SPRING_STIFFNESS,
            damping: SPRING_DAMPING,
        }) {
            Some(h) => h,
            None => return,
        };

        // Create visual bond entity (capsule SDF — no physics body)
        let bond_id = ctx.next_id();
        let midpoint = (pos_a + pos_b) / 2.0;
        let half_len = rest_length / 2.0;
        let angle = (pos_b.y - pos_a.y).atan2(pos_b.x - pos_a.x) - std::f32::consts::FRAC_PI_2;

        let entity = Entity::new(bond_id)
            .with_tag("bond")
            .with_pos(midpoint)
            .with_rotation(angle)
            .with_mesh(MeshComponent::capsule(BOND_RADIUS, half_len, BOND_COLOR));

        ctx.scene.spawn(entity);

        self.molecule.add_bond(Bond {
            atom_a,
            atom_b,
            visual_entity: bond_id,
            joint_handle,
        });
    }

    fn update_bond_visuals(&self, ctx: &mut EngineContext) {
        for bond in &self.molecule.bonds {
            let pos_a = match ctx.scene.get(bond.atom_a) {
                Some(e) => e.pos,
                None => continue,
            };
            let pos_b = match ctx.scene.get(bond.atom_b) {
                Some(e) => e.pos,
                None => continue,
            };

            let midpoint = (pos_a + pos_b) / 2.0;
            let distance = pos_a.distance(pos_b);
            let half_len = distance / 2.0;
            let angle = (pos_b.y - pos_a.y).atan2(pos_b.x - pos_a.x) - std::f32::consts::FRAC_PI_2;

            if let Some(entity) = ctx.scene.get_mut(bond.visual_entity) {
                entity.pos = midpoint;
                entity.rotation = angle;
                // Update capsule half_height to match current distance
                if let Some(ref mut mesh) = entity.mesh {
                    mesh.shape = zap_engine::components::mesh::SDFShape::Capsule {
                        radius: BOND_RADIUS,
                        half_height: half_len,
                    };
                }
            }
        }
    }

    fn clear_all(&mut self, ctx: &mut EngineContext) {
        // Remove all joints
        for bond in &self.molecule.bonds {
            ctx.remove_joint(bond.joint_handle);
            // Remove visual entity
            if ctx.scene.get(bond.visual_entity).is_some() {
                ctx.scene.despawn(bond.visual_entity);
            }
        }

        // Remove all atoms
        for &(atom_id, _, _) in &self.molecule.atoms {
            ctx.despawn(atom_id);
        }

        self.molecule = MoleculeState::new();
        self.drag_from = None;
        self.drag_active = false;
    }
}

impl Game for ChemistryLab {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: 1.0 / 60.0,
            world_width: WORLD_W,
            world_height: WORLD_H,
            max_sdf_instances: 256,
            gravity: Vec2::new(0.0, 50.0),
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        // Spawn boundary walls
        let hw = WORLD_W / 2.0;
        let hh = WORLD_H / 2.0;
        let wt = WALL_THICKNESS / 2.0;

        // Top
        Self::spawn_wall(ctx, Vec2::new(hw, -wt), hw + WALL_THICKNESS, wt);
        // Bottom
        Self::spawn_wall(ctx, Vec2::new(hw, WORLD_H + wt), hw + WALL_THICKNESS, wt);
        // Left
        Self::spawn_wall(ctx, Vec2::new(-wt, hh), wt, hh + WALL_THICKNESS);
        // Right
        Self::spawn_wall(ctx, Vec2::new(WORLD_W + wt, hh), wt, hh + WALL_THICKNESS);
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        for event in input.iter() {
            match event {
                InputEvent::Custom { kind, a, .. } => {
                    if *kind == CUSTOM_SELECT_ELEMENT {
                        self.selected_element = *a as u32;
                    } else if *kind == CUSTOM_CLEAR {
                        self.clear_all(ctx);
                    }
                }
                InputEvent::PointerDown { x, y } => {
                    let pos = Vec2::new(*x, *y);
                    if let Some(hit_id) = self.hit_test_atom(ctx, pos) {
                        // Start bond drag from existing atom
                        self.drag_from = Some(hit_id);
                        self.drag_active = true;
                    } else {
                        // Spawn new atom at click position
                        self.spawn_atom(ctx, pos);
                    }
                }
                InputEvent::PointerUp { x, y } => {
                    if self.drag_active {
                        if let Some(from_id) = self.drag_from.take() {
                            let pos = Vec2::new(*x, *y);
                            if let Some(target_id) = self.hit_test_atom(ctx, pos) {
                                self.try_create_bond(ctx, from_id, target_id);
                            }
                        }
                        self.drag_active = false;
                    }
                }
                _ => {}
            }
        }

        // Update bond visuals to match physics positions
        self.update_bond_visuals(ctx);

        // Emit stats to React
        ctx.emit_event(GameEvent {
            kind: EVENT_ATOM_COUNT,
            a: self.molecule.atoms.len() as f32,
            b: 0.0,
            c: 0.0,
        });
        ctx.emit_event(GameEvent {
            kind: EVENT_BOND_COUNT,
            a: self.molecule.bonds.len() as f32,
            b: 0.0,
            c: 0.0,
        });

        ctx.effects.attractor = [WORLD_W / 2.0, WORLD_H / 2.0];
    }
}
