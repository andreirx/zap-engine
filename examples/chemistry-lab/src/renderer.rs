//! Visualization renderer for the chemistry simulation.
//!
//! Converts simulation state (MoleculeState3D) to engine visuals (SDF instances).

use glam::Vec2;
use zap_engine::components::mesh::{MeshComponent, SDFColor};
use zap_engine::{Entity, EntityId, EngineContext};

use crate::math3d::Camera3D;
use crate::molecule3d::MoleculeState3D;
use crate::periodic_table::ElementRegistry;
use crate::render3d::build_3d_sdf_buffer;

/// Visual effects state for the current frame.
pub struct FrameEffects {
    /// Draw a bond preview arc from atom to pointer.
    pub bond_preview: Option<BondPreview>,
}

/// Bond preview visualization.
pub struct BondPreview {
    pub from_atom_idx: usize,
    pub to_screen_pos: Vec2,
}

/// Renderer that syncs simulation state to engine visuals.
pub struct MoleculeRenderer {
    /// Base entity ID for rendered instances.
    base_entity_id: u32,
    /// List of currently active entity IDs to reuse.
    active_ids: Vec<EntityId>,
}

impl MoleculeRenderer {
    pub fn new() -> Self {
        Self {
            base_entity_id: 10000,
            active_ids: Vec::with_capacity(128),
        }
    }

    /// Synchronize visuals with the current simulation state.
    pub fn sync_visuals(
        &mut self,
        ctx: &mut EngineContext,
        molecule: &MoleculeState3D,
        camera: &Camera3D,
        registry: &ElementRegistry,
        effects: &FrameEffects,
    ) {
        // Build SDF instances from 3D molecule state
        let instances = build_3d_sdf_buffer(molecule, camera, registry);
        let needed_count = instances.len();

        // 1. Update existing entities
        let update_count = self.active_ids.len().min(needed_count);
        for i in 0..update_count {
            let id = self.active_ids[i];
            let inst = &instances[i];
            
            if let Some(entity) = ctx.scene.get_mut(id) {
                entity.pos = Vec2::new(inst.x, inst.y);
                entity.rotation = inst.rotation;
                
                // Update mesh uniforms (color, radius, shape)
                if let Some(mesh) = &mut entity.mesh {
                    // Update mesh properties directly if possible, or replace
                    // MeshComponent struct is simple data, cheap to replace
                     let new_mesh = if inst.shape_type < 0.5 {
                        MeshComponent::sphere(
                            inst.radius,
                            SDFColor::new(inst.r, inst.g, inst.b),
                        )
                    } else {
                        MeshComponent::capsule(
                            inst.radius,
                            inst.half_height,
                            SDFColor::new(inst.r, inst.g, inst.b),
                        )
                    };
                    *mesh = new_mesh;
                }
            }
        }

        // 2. Spawn new entities if needed
        if needed_count > self.active_ids.len() {
            for i in self.active_ids.len()..needed_count {
                let inst = &instances[i];
                let id = EntityId(self.base_entity_id + i as u32);
                
                let mesh = if inst.shape_type < 0.5 {
                    MeshComponent::sphere(
                        inst.radius,
                        SDFColor::new(inst.r, inst.g, inst.b),
                    )
                } else {
                    MeshComponent::capsule(
                        inst.radius,
                        inst.half_height,
                        SDFColor::new(inst.r, inst.g, inst.b),
                    )
                };

                let entity = Entity::new(id)
                    .with_pos(Vec2::new(inst.x, inst.y))
                    .with_rotation(inst.rotation)
                    .with_mesh(mesh);

                ctx.scene.spawn(entity);
                self.active_ids.push(id);
            }
        }

        // 3. Despawn excess entities
        if self.active_ids.len() > needed_count {
            for i in (needed_count..self.active_ids.len()).rev() {
                let id = self.active_ids[i];
                ctx.scene.despawn(id);
            }
            self.active_ids.truncate(needed_count);
        }

        // Render bond preview if dragging
        if let Some(preview) = &effects.bond_preview {
            self.render_bond_preview(ctx, molecule, camera, preview);
        }
    }

    /// Render the bond preview arc.
    fn render_bond_preview(
        &self,
        ctx: &mut EngineContext,
        molecule: &MoleculeState3D,
        camera: &Camera3D,
        preview: &BondPreview,
    ) {
        if let Some(atom) = molecule.atoms.get(preview.from_atom_idx) {
            let proj = camera.project(atom.position);
            let start = [proj.pos.x, proj.pos.y];
            let end = [preview.to_screen_pos.x, preview.to_screen_pos.y];

            ctx.effects.add_arc(
                start,
                end,
                3.0,
                zap_engine::systems::effects::SegmentColor::Cyan,
                4,
            );
        }
    }
}

impl Default for MoleculeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_creation() {
        let renderer = MoleculeRenderer::new();
        assert_eq!(renderer.base_entity_id, 10000);
    }
}
