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
        let (radius, shape_type, half_height, extra) = match mesh.shape {
            SDFShape::Sphere { radius } => (radius, 0.0, 0.0, 0.0),
            SDFShape::Capsule { radius, half_height } => (radius, 1.0, half_height, 0.0),
            SDFShape::RoundedBox { radius, half_height, corner_radius } => (radius, 2.0, half_height, corner_radius),
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
            shape_type,
            half_height,
            extra,
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
    fn build_sdf_buffer_capsule() {
        let entity = Entity::new(EntityId(1))
            .with_pos(Vec2::new(100.0, 200.0))
            .with_rotation(1.57)
            .with_mesh(
                MeshComponent::capsule(5.0, 20.0, SDFColor::new(0.0, 1.0, 0.0)),
            );

        let entities = vec![entity];
        let mut buffer = SDFBuffer::new();
        build_sdf_buffer(entities.iter(), &mut buffer);
        assert_eq!(buffer.instance_count(), 1);

        let ptr = buffer.instances_ptr();
        unsafe {
            assert_eq!(*ptr.add(2), 5.0);   // radius
            assert_eq!(*ptr.add(3), 1.57);  // rotation
            assert_eq!(*ptr.add(9), 1.0);   // shape_type = Capsule
            assert_eq!(*ptr.add(10), 20.0); // half_height
            assert_eq!(*ptr.add(11), 0.0);  // extra
        }
    }

    #[test]
    fn build_sdf_buffer_rounded_box() {
        let entity = Entity::new(EntityId(1))
            .with_pos(Vec2::ZERO)
            .with_mesh(
                MeshComponent::rounded_box(15.0, 10.0, 3.0, SDFColor::new(1.0, 1.0, 1.0)),
            );

        let entities = vec![entity];
        let mut buffer = SDFBuffer::new();
        build_sdf_buffer(entities.iter(), &mut buffer);
        assert_eq!(buffer.instance_count(), 1);

        let ptr = buffer.instances_ptr();
        unsafe {
            assert_eq!(*ptr.add(2), 15.0);  // radius
            assert_eq!(*ptr.add(9), 2.0);   // shape_type = RoundedBox
            assert_eq!(*ptr.add(10), 10.0); // half_height
            assert_eq!(*ptr.add(11), 3.0);  // extra = corner_radius
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
