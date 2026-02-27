// extensions/transform.rs
//
// Transform hierarchy extension — tracks parent-child relationships by EntityId.
// Completely decoupled from Entity/Scene internals.
//
// Usage:
//   let mut graph = TransformGraph::new();
//   graph.set_parent(child_id, parent_id);
//   graph.propagate(&mut scene);  // Updates world positions from local offsets

use std::collections::HashMap;
use glam::Vec2;
use crate::api::types::EntityId;
use crate::core::scene::Scene;

/// Local transform data for entities in a hierarchy.
#[derive(Debug, Clone, Copy)]
pub struct LocalTransform {
    /// Position relative to parent (or world if no parent).
    pub offset: Vec2,
    /// Rotation relative to parent.
    pub rotation: f32,
    /// Scale multiplier relative to parent.
    pub scale: Vec2,
}

impl Default for LocalTransform {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }
}

impl LocalTransform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
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
}

/// Node in the transform hierarchy.
#[derive(Debug, Clone)]
struct TransformNode {
    parent: Option<EntityId>,
    children: Vec<EntityId>,
    local: LocalTransform,
}

impl Default for TransformNode {
    fn default() -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            local: LocalTransform::default(),
        }
    }
}

/// Transform hierarchy graph — manages parent-child relationships.
///
/// Exists separately from Scene to maintain clean architecture.
/// Games that need hierarchy create this alongside their Scene.
#[derive(Debug, Default)]
pub struct TransformGraph {
    nodes: HashMap<EntityId, TransformNode>,
    /// Entities with no parent (top-level).
    roots: Vec<EntityId>,
    /// Dirty flag — set when hierarchy changes, cleared after propagate.
    dirty: bool,
}

impl TransformGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an entity in the hierarchy with default local transform.
    /// Must be called before setting parent/children.
    pub fn register(&mut self, id: EntityId) {
        self.nodes.entry(id).or_default();
        if !self.roots.contains(&id) {
            self.roots.push(id);
        }
        self.dirty = true;
    }

    /// Register an entity with a specific local transform.
    pub fn register_with(&mut self, id: EntityId, local: LocalTransform) {
        let node = self.nodes.entry(id).or_default();
        node.local = local;
        if !self.roots.contains(&id) {
            self.roots.push(id);
        }
        self.dirty = true;
    }

    /// Set the parent of an entity. Pass `None` to make it a root.
    pub fn set_parent(&mut self, child: EntityId, parent: Option<EntityId>) {
        // Ensure both exist
        self.nodes.entry(child).or_default();
        if let Some(p) = parent {
            self.nodes.entry(p).or_default();
        }

        // Remove from old parent's children
        if let Some(old_parent) = self.nodes.get(&child).and_then(|n| n.parent) {
            if let Some(old_node) = self.nodes.get_mut(&old_parent) {
                old_node.children.retain(|&c| c != child);
            }
        }

        // Update child's parent
        if let Some(node) = self.nodes.get_mut(&child) {
            node.parent = parent;
        }

        // Add to new parent's children
        if let Some(p) = parent {
            if let Some(parent_node) = self.nodes.get_mut(&p) {
                if !parent_node.children.contains(&child) {
                    parent_node.children.push(child);
                }
            }
            // Remove from roots if it has a parent now
            self.roots.retain(|&r| r != child);
        } else {
            // No parent — add to roots
            if !self.roots.contains(&child) {
                self.roots.push(child);
            }
        }

        self.dirty = true;
    }

    /// Set the local transform for an entity.
    pub fn set_local(&mut self, id: EntityId, local: LocalTransform) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.local = local;
            self.dirty = true;
        }
    }

    /// Get the local transform for an entity.
    pub fn get_local(&self, id: EntityId) -> Option<&LocalTransform> {
        self.nodes.get(&id).map(|n| &n.local)
    }

    /// Get the local transform mutably.
    pub fn get_local_mut(&mut self, id: EntityId) -> Option<&mut LocalTransform> {
        self.dirty = true;
        self.nodes.get_mut(&id).map(|n| &mut n.local)
    }

    /// Get the parent of an entity.
    pub fn get_parent(&self, id: EntityId) -> Option<EntityId> {
        self.nodes.get(&id).and_then(|n| n.parent)
    }

    /// Get the children of an entity.
    pub fn get_children(&self, id: EntityId) -> Option<&[EntityId]> {
        self.nodes.get(&id).map(|n| n.children.as_slice())
    }

    /// Remove an entity from the hierarchy.
    /// Children become roots (orphaned).
    pub fn remove(&mut self, id: EntityId) {
        if let Some(node) = self.nodes.remove(&id) {
            // Remove from parent's children
            if let Some(parent) = node.parent {
                if let Some(parent_node) = self.nodes.get_mut(&parent) {
                    parent_node.children.retain(|&c| c != id);
                }
            }

            // Orphan children (make them roots)
            for child in node.children {
                if let Some(child_node) = self.nodes.get_mut(&child) {
                    child_node.parent = None;
                }
                if !self.roots.contains(&child) {
                    self.roots.push(child);
                }
            }

            // Remove from roots
            self.roots.retain(|&r| r != id);
        }
        self.dirty = true;
    }

    /// Propagate transforms from roots down through the hierarchy.
    /// Updates Entity.pos/rotation/scale based on parent transforms.
    pub fn propagate(&mut self, scene: &mut Scene) {
        if !self.dirty {
            return;
        }

        // Process roots first, then children recursively
        let roots: Vec<EntityId> = self.roots.clone();
        for root in roots {
            self.propagate_recursive(root, Vec2::ZERO, 0.0, Vec2::ONE, scene);
        }

        self.dirty = false;
    }

    fn propagate_recursive(
        &self,
        id: EntityId,
        parent_pos: Vec2,
        parent_rot: f32,
        parent_scale: Vec2,
        scene: &mut Scene,
    ) {
        let Some(node) = self.nodes.get(&id) else { return };
        let local = &node.local;

        // Compute world transform
        // Rotate the offset by parent rotation, then scale and translate
        let cos_r = parent_rot.cos();
        let sin_r = parent_rot.sin();
        let rotated_offset = Vec2::new(
            local.offset.x * cos_r - local.offset.y * sin_r,
            local.offset.x * sin_r + local.offset.y * cos_r,
        );
        let world_pos = parent_pos + rotated_offset * parent_scale;
        let world_rot = parent_rot + local.rotation;
        let world_scale = parent_scale * local.scale;

        // Update entity in scene
        if let Some(entity) = scene.get_mut(id) {
            entity.pos = world_pos;
            entity.rotation = world_rot;
            entity.scale = world_scale;
        }

        // Propagate to children
        let children: Vec<EntityId> = node.children.clone();
        for child in children {
            self.propagate_recursive(child, world_pos, world_rot, world_scale, scene);
        }
    }

    /// Check if the hierarchy has pending changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the hierarchy as needing propagation.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Number of entities in the hierarchy.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the hierarchy is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Clear all hierarchy data.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.roots.clear();
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::entity::Entity;

    #[test]
    fn parent_child_relationship() {
        let mut graph = TransformGraph::new();
        let parent = EntityId(1);
        let child = EntityId(2);

        graph.register(parent);
        graph.register(child);
        graph.set_parent(child, Some(parent));

        assert_eq!(graph.get_parent(child), Some(parent));
        assert_eq!(graph.get_children(parent), Some([child].as_slice()));
    }

    #[test]
    fn propagate_updates_positions() {
        let mut graph = TransformGraph::new();
        let mut scene = Scene::new();

        let parent = EntityId(1);
        let child = EntityId(2);

        scene.spawn(Entity::new(parent).with_pos(Vec2::new(100.0, 100.0)));
        scene.spawn(Entity::new(child));

        graph.register_with(parent, LocalTransform::new().with_offset(Vec2::new(100.0, 100.0)));
        graph.register_with(child, LocalTransform::new().with_offset(Vec2::new(50.0, 0.0)));
        graph.set_parent(child, Some(parent));

        graph.propagate(&mut scene);

        let child_entity = scene.get(child).unwrap();
        assert_eq!(child_entity.pos, Vec2::new(150.0, 100.0));
    }

    #[test]
    fn remove_orphans_children() {
        let mut graph = TransformGraph::new();
        let parent = EntityId(1);
        let child = EntityId(2);

        graph.register(parent);
        graph.register(child);
        graph.set_parent(child, Some(parent));

        graph.remove(parent);

        assert_eq!(graph.get_parent(child), None);
        assert!(graph.roots.contains(&child));
    }
}
