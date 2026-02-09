use crate::api::types::EntityId;
use crate::components::entity::Entity;

/// Simple entity storage using a flat Vec.
/// Designed for small-to-medium entity counts (hundreds, not millions).
pub struct Scene {
    entities: Vec<Entity>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            entities: Vec::with_capacity(256),
        }
    }

    /// Create a scene with a specific entity capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: Vec::with_capacity(capacity),
        }
    }

    /// Add an entity to the scene.
    pub fn spawn(&mut self, entity: Entity) {
        self.entities.push(entity);
    }

    /// Remove an entity by ID. Returns the removed entity if found.
    pub fn despawn(&mut self, id: EntityId) -> Option<Entity> {
        if let Some(idx) = self.entities.iter().position(|e| e.id == id) {
            Some(self.entities.swap_remove(idx))
        } else {
            None
        }
    }

    /// Get a reference to an entity by ID.
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Get a mutable reference to an entity by ID.
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    /// Iterate over all entities.
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    /// Iterate over all entities mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.entities.iter_mut()
    }

    /// Find the first entity with the given tag.
    pub fn find_by_tag(&self, tag: &str) -> Option<&Entity> {
        self.entities.iter().find(|e| e.tag == tag)
    }

    /// Find the first entity with the given tag (mutable).
    pub fn find_by_tag_mut(&mut self, tag: &str) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.tag == tag)
    }

    /// Find all entities with the given tag.
    pub fn find_all_by_tag(&self, tag: &str) -> Vec<&Entity> {
        self.entities.iter().filter(|e| e.tag == tag).collect()
    }

    /// Number of entities in the scene.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Whether the scene is empty.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Clear all entities.
    pub fn clear(&mut self) {
        self.entities.clear();
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn spawn_and_get() {
        let mut scene = Scene::new();
        let id = EntityId(1);
        scene.spawn(Entity::new(id).with_pos(Vec2::new(10.0, 20.0)));
        let e = scene.get(id).unwrap();
        assert_eq!(e.pos, Vec2::new(10.0, 20.0));
    }

    #[test]
    fn despawn_removes_entity() {
        let mut scene = Scene::new();
        let id = EntityId(1);
        scene.spawn(Entity::new(id));
        assert_eq!(scene.len(), 1);
        scene.despawn(id);
        assert_eq!(scene.len(), 0);
    }

    #[test]
    fn find_by_tag() {
        let mut scene = Scene::new();
        scene.spawn(Entity::new(EntityId(1)).with_tag("hero"));
        scene.spawn(Entity::new(EntityId(2)).with_tag("enemy"));
        let hero = scene.find_by_tag("hero").unwrap();
        assert_eq!(hero.id, EntityId(1));
    }
}
