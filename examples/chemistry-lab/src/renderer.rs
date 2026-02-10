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
}

impl MoleculeRenderer {
    pub fn new() -> Self {
        Self {
            base_entity_id: 10000,
        }
    }

    /// Synchronize visuals with the current simulation state.
    pub fn sync_visuals(
        &self,
        ctx: &mut EngineContext,
        molecule: &MoleculeState3D,
        camera: &Camera3D,
        registry: &ElementRegistry,
        effects: &FrameEffects,
    ) {
        // Clear existing mesh entities
        self.clear_meshes(ctx);

        // Build SDF instances from 3D molecule state
        let instances = build_3d_sdf_buffer(molecule, camera, registry);

        // Create entities for each SDF instance
        for (i, inst) in instances.iter().enumerate() {
            let id = EntityId(self.base_entity_id + i as u32);

            let mesh = if inst.shape_type < 0.5 {
                // Sphere (atom)
                MeshComponent::sphere(
                    inst.radius,
                    SDFColor::new(inst.r, inst.g, inst.b),
                )
            } else {
                // Capsule (bond)
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
        }

        // Render bond preview if dragging
        if let Some(preview) = &effects.bond_preview {
            self.render_bond_preview(ctx, molecule, camera, preview);
        }
    }

    /// Clear all mesh entities from the scene.
    fn clear_meshes(&self, ctx: &mut EngineContext) {
        let to_remove: Vec<EntityId> = ctx.scene.iter()
            .filter(|e| e.mesh.is_some())
            .map(|e| e.id)
            .collect();

        for id in to_remove {
            ctx.scene.despawn(id);
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
