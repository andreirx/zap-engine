use crate::api::types::EntityId;
use crate::components::entity::Entity;
use std::collections::HashMap;

/// Entity storage with O(1) ID lookups via HashMap index.
/// Designed for small-to-medium entity counts (hundreds, not millions).
/// The Vec provides cache-friendly iteration; the HashMap provides fast random access.
pub struct Scene {
    entities: Vec<Entity>,
    /// Maps EntityId → index in entities Vec for O(1) lookup
    id_index: HashMap<EntityId, usize>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            entities: Vec::with_capacity(256),
            id_index: HashMap::with_capacity(256),
        }
    }

    /// Create a scene with a specific entity capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: Vec::with_capacity(capacity),
            id_index: HashMap::with_capacity(capacity),
        }
    }

    /// Add an entity to the scene.
    pub fn spawn(&mut self, entity: Entity) {
        let id = entity.id;
        let idx = self.entities.len();
        self.entities.push(entity);
        self.id_index.insert(id, idx);
    }

    /// Remove an entity by ID. Returns the removed entity if found.
    /// Uses swap_remove for O(1) removal, which changes the order of entities.
    pub fn despawn(&mut self, id: EntityId) -> Option<Entity> {
        if let Some(&idx) = self.id_index.get(&id) {
            self.id_index.remove(&id);
            let removed = self.entities.swap_remove(idx);
            // Update the index of the entity that was swapped into this position
            if idx < self.entities.len() {
                let swapped_id = self.entities[idx].id;
                self.id_index.insert(swapped_id, idx);
            }
            Some(removed)
        } else {
            None
        }
    }

    /// Get a reference to an entity by ID. O(1) via HashMap index.
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.id_index.get(&id).map(|&idx| &self.entities[idx])
    }

    /// Get a mutable reference to an entity by ID. O(1) via HashMap index.
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        if let Some(&idx) = self.id_index.get(&id) {
            Some(&mut self.entities[idx])
        } else {
            None
        }
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

    /// Retain only entities matching the predicate. Preserves order.
    /// More efficient than multiple despawn calls when removing many entities.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&Entity) -> bool,
    {
        // Rebuild index after retain
        self.entities.retain(|e| f(e));
        self.rebuild_index();
    }

    /// Remove all entities with the given tag. Preserves order.
    pub fn despawn_by_tag(&mut self, tag: &str) {
        self.retain(|e| e.tag != tag);
    }

    /// Rebuild the ID index from the entities Vec.
    /// Called after operations that invalidate indices (retain, etc).
    fn rebuild_index(&mut self) {
        self.id_index.clear();
        for (idx, entity) in self.entities.iter().enumerate() {
            self.id_index.insert(entity.id, idx);
        }
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
        self.id_index.clear();
    }

    /// Check if an entity with the given ID exists. O(1).
    pub fn contains(&self, id: EntityId) -> bool {
        self.id_index.contains_key(&id)
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
