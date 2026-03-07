// extensions/spatial.rs
//
// Spatial hash grid for efficient "find entities near point" queries.
// Ideal for RTS unit selection, collision broad-phase, proximity triggers.
//
// Usage:
//   let mut grid = SpatialHash::new(64.0); // 64 world-unit cells
//   grid.insert(entity_id, position);
//   let nearby = grid.query_rect(min, max); // O(1) per cell touched
//
// Games must manually update positions when entities move.
// Call grid.clear() + re-insert all, or use update() for individual moves.

use crate::api::types::EntityId;
use glam::Vec2;
use std::collections::HashMap;

/// Spatial hash grid for O(1) proximity queries.
/// Cell size should match typical query radius (e.g., unit selection box, attack range).
pub struct SpatialHash {
    cell_size: f32,
    inv_cell_size: f32,
    /// Maps cell (ix, iy) to list of entity IDs in that cell
    cells: HashMap<(i32, i32), Vec<EntityId>>,
    /// Maps entity ID to its current cell (for fast updates)
    entity_cells: HashMap<EntityId, (i32, i32)>,
}

impl SpatialHash {
    /// Create a new spatial hash with the given cell size.
    /// Smaller cells = more precise but more memory. Larger = fewer cells but more entities per cell.
    /// Rule of thumb: cell_size ≈ typical query radius or unit spacing.
    pub fn new(cell_size: f32) -> Self {
        assert!(cell_size > 0.0, "cell_size must be positive");
        Self {
            cell_size,
            inv_cell_size: 1.0 / cell_size,
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    /// Create with expected capacity for better performance.
    pub fn with_capacity(cell_size: f32, entity_capacity: usize) -> Self {
        assert!(cell_size > 0.0, "cell_size must be positive");
        Self {
            cell_size,
            inv_cell_size: 1.0 / cell_size,
            cells: HashMap::with_capacity(entity_capacity / 4), // Assume ~4 entities per cell
            entity_cells: HashMap::with_capacity(entity_capacity),
        }
    }

    /// Convert world position to cell coordinates.
    #[inline]
    fn pos_to_cell(&self, pos: Vec2) -> (i32, i32) {
        (
            (pos.x * self.inv_cell_size).floor() as i32,
            (pos.y * self.inv_cell_size).floor() as i32,
        )
    }

    /// Insert an entity at a position. If already present, updates its position.
    pub fn insert(&mut self, id: EntityId, pos: Vec2) {
        let new_cell = self.pos_to_cell(pos);

        // Remove from old cell if present
        if let Some(&old_cell) = self.entity_cells.get(&id) {
            if old_cell == new_cell {
                return; // Same cell, no change needed
            }
            if let Some(cell_entities) = self.cells.get_mut(&old_cell) {
                cell_entities.retain(|&e| e != id);
            }
        }

        // Add to new cell
        self.cells.entry(new_cell).or_default().push(id);
        self.entity_cells.insert(id, new_cell);
    }

    /// Update an entity's position (alias for insert).
    #[inline]
    pub fn update(&mut self, id: EntityId, pos: Vec2) {
        self.insert(id, pos);
    }

    /// Remove an entity from the grid.
    pub fn remove(&mut self, id: EntityId) {
        if let Some(cell) = self.entity_cells.remove(&id) {
            if let Some(cell_entities) = self.cells.get_mut(&cell) {
                cell_entities.retain(|&e| e != id);
            }
        }
    }

    /// Clear all entities from the grid.
    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_cells.clear();
    }

    /// Query all entities within a rectangle (axis-aligned bounding box).
    /// Returns entity IDs; caller looks up positions/components as needed.
    pub fn query_rect(&self, min: Vec2, max: Vec2) -> Vec<EntityId> {
        let min_cell = self.pos_to_cell(min);
        let max_cell = self.pos_to_cell(max);

        let mut result = Vec::new();

        for ix in min_cell.0..=max_cell.0 {
            for iy in min_cell.1..=max_cell.1 {
                if let Some(cell_entities) = self.cells.get(&(ix, iy)) {
                    result.extend(cell_entities.iter().copied());
                }
            }
        }

        result
    }

    /// Query all entities within a radius of a point.
    /// First finds candidates via rect query, then filters by actual distance.
    pub fn query_radius(&self, center: Vec2, radius: f32, positions: &impl Fn(EntityId) -> Option<Vec2>) -> Vec<EntityId> {
        let r2 = radius * radius;
        let min = center - Vec2::splat(radius);
        let max = center + Vec2::splat(radius);

        self.query_rect(min, max)
            .into_iter()
            .filter(|&id| {
                if let Some(pos) = positions(id) {
                    (pos - center).length_squared() <= r2
                } else {
                    false
                }
            })
            .collect()
    }

    /// Query entities in rect, with exact position filtering.
    /// Useful when rect query is approximate (cell boundaries).
    pub fn query_rect_exact(&self, min: Vec2, max: Vec2, positions: &impl Fn(EntityId) -> Option<Vec2>) -> Vec<EntityId> {
        self.query_rect(min, max)
            .into_iter()
            .filter(|&id| {
                if let Some(pos) = positions(id) {
                    pos.x >= min.x && pos.x <= max.x && pos.y >= min.y && pos.y <= max.y
                } else {
                    false
                }
            })
            .collect()
    }

    /// Number of entities in the grid.
    pub fn len(&self) -> usize {
        self.entity_cells.len()
    }

    /// Whether the grid is empty.
    pub fn is_empty(&self) -> bool {
        self.entity_cells.is_empty()
    }

    /// Number of non-empty cells.
    pub fn cell_count(&self) -> usize {
        self.cells.values().filter(|v| !v.is_empty()).count()
    }

    /// Check if an entity is in the grid.
    pub fn contains(&self, id: EntityId) -> bool {
        self.entity_cells.contains_key(&id)
    }

    /// Get the cell an entity is in (if present).
    pub fn get_cell(&self, id: EntityId) -> Option<(i32, i32)> {
        self.entity_cells.get(&id).copied()
    }

    /// Get the cell size.
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_query() {
        let mut grid = SpatialHash::new(10.0);

        grid.insert(EntityId(1), Vec2::new(5.0, 5.0));   // Cell (0, 0)
        grid.insert(EntityId(2), Vec2::new(15.0, 5.0));  // Cell (1, 0)
        grid.insert(EntityId(3), Vec2::new(25.0, 5.0));  // Cell (2, 0)

        // Query covering cells (0,0) and (1,0) only — stops before cell 2
        let result = grid.query_rect(Vec2::new(0.0, 0.0), Vec2::new(19.9, 10.0));
        assert_eq!(result.len(), 2);
        assert!(result.contains(&EntityId(1)));
        assert!(result.contains(&EntityId(2)));

        // Query covering all three cells
        let result_all = grid.query_rect(Vec2::new(0.0, 0.0), Vec2::new(30.0, 10.0));
        assert_eq!(result_all.len(), 3);
    }

    #[test]
    fn update_moves_entity() {
        let mut grid = SpatialHash::new(10.0);

        grid.insert(EntityId(1), Vec2::new(5.0, 5.0));
        assert_eq!(grid.get_cell(EntityId(1)), Some((0, 0)));

        grid.update(EntityId(1), Vec2::new(15.0, 5.0));
        assert_eq!(grid.get_cell(EntityId(1)), Some((1, 0)));

        // Old cell should not contain it
        let result = grid.query_rect(Vec2::new(0.0, 0.0), Vec2::new(9.0, 10.0));
        assert!(result.is_empty());
    }

    #[test]
    fn remove_entity() {
        let mut grid = SpatialHash::new(10.0);

        grid.insert(EntityId(1), Vec2::new(5.0, 5.0));
        assert_eq!(grid.len(), 1);

        grid.remove(EntityId(1));
        assert_eq!(grid.len(), 0);
        assert!(!grid.contains(EntityId(1)));
    }

    #[test]
    fn query_radius() {
        let mut grid = SpatialHash::new(10.0);

        grid.insert(EntityId(1), Vec2::new(0.0, 0.0));
        grid.insert(EntityId(2), Vec2::new(5.0, 0.0));
        grid.insert(EntityId(3), Vec2::new(15.0, 0.0));

        // Positions lookup
        let positions = |id: EntityId| match id.0 {
            1 => Some(Vec2::new(0.0, 0.0)),
            2 => Some(Vec2::new(5.0, 0.0)),
            3 => Some(Vec2::new(15.0, 0.0)),
            _ => None,
        };

        // Radius 6 from origin should include entities 1 and 2
        let result = grid.query_radius(Vec2::ZERO, 6.0, &positions);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&EntityId(1)));
        assert!(result.contains(&EntityId(2)));
    }

    #[test]
    fn negative_coordinates() {
        let mut grid = SpatialHash::new(10.0);

        grid.insert(EntityId(1), Vec2::new(-5.0, -5.0));
        grid.insert(EntityId(2), Vec2::new(-15.0, -5.0));

        let result = grid.query_rect(Vec2::new(-20.0, -10.0), Vec2::new(0.0, 0.0));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn same_cell_no_duplicate() {
        let mut grid = SpatialHash::new(10.0);

        grid.insert(EntityId(1), Vec2::new(5.0, 5.0));
        grid.insert(EntityId(1), Vec2::new(6.0, 6.0)); // Same cell

        assert_eq!(grid.len(), 1);
        let result = grid.query_rect(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        assert_eq!(result.len(), 1);
    }
}
