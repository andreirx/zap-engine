use crate::components::entity::Entity;
use crate::components::mesh::SDFShape;
use crate::renderer::sdf_instance::{SDFBuffer, SDFInstance};

/// Build the SDF instance buffer from entities with mesh components.
pub fn build_sdf_buffer<'a>(
    entities: impl Iterator<Item = &'a Entity>,
    buffer: &mut SDFBuffer,
) {
    buffer.clear();
    for entity in entities {
        if !entity.active {
            continue;
        }
        let mesh = match &entity.mesh {
            Some(m) => m,
            None => continue,
        };
        let radius = match mesh.shape {
            SDFShape::Sphere { radius } => radius,
        };
        buffer.push(SDFInstance {
            x: entity.pos.x,
            y: entity.pos.y,
            radius,
            rotation: entity.rotation,
            r: mesh.color.r,
            g: mesh.color.g,
            b: mesh.color.b,
            shininess: mesh.shininess,
            emissive: mesh.emissive,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::EntityId;
    use crate::components::mesh::{MeshComponent, SDFColor, SDFShape};
    use glam::Vec2;

    #[test]
    fn build_sdf_buffer_from_entity_with_mesh() {
        let entity = Entity::new(EntityId(1))
            .with_pos(Vec2::new(50.0, 75.0))
            .with_mesh(
                MeshComponent::new(
                    SDFShape::Sphere { radius: 15.0 },
                    SDFColor::new(1.0, 0.0, 0.0),
                )
                .with_shininess(64.0)
                .with_emissive(0.5),
            );

        let entities = vec![entity];
        let mut buffer = SDFBuffer::new();
        build_sdf_buffer(entities.iter(), &mut buffer);

        assert_eq!(buffer.instance_count(), 1);

        // Read back via pointer
        let ptr = buffer.instances_ptr();
        unsafe {
            let x = *ptr;
            let y = *ptr.add(1);
            let radius = *ptr.add(2);
            let r = *ptr.add(4);
            let shininess = *ptr.add(7);
            let emissive = *ptr.add(8);
            assert_eq!(x, 50.0);
            assert_eq!(y, 75.0);
            assert_eq!(radius, 15.0);
            assert_eq!(r, 1.0);
            assert_eq!(shininess, 64.0);
            assert_eq!(emissive, 0.5);
        }
    }

    #[test]
    fn build_sdf_buffer_skips_inactive_and_no_mesh() {
        let e1 = Entity::new(EntityId(1)); // no mesh
        let mut e2 = Entity::new(EntityId(2))
            .with_mesh(MeshComponent::default());
        e2.active = false; // inactive
        let e3 = Entity::new(EntityId(3))
            .with_mesh(MeshComponent::default()); // should appear

        let entities = vec![e1, e2, e3];
        let mut buffer = SDFBuffer::new();
        build_sdf_buffer(entities.iter(), &mut buffer);

        assert_eq!(buffer.instance_count(), 1);
    }
}
