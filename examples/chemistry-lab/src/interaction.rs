//! Input handling and interaction system for the chemistry simulation.
//!
//! Handles pointer events, hit testing, and user interaction modes.

use glam::Vec2;
use crate::math3d::{Camera3D, Vec3};
use crate::molecule3d::MoleculeState3D;
use crate::periodic_table::ElementRegistry;
use crate::render3d::{hit_test_2d, screen_to_world_on_plane};

/// The current interaction mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionMode {
    /// No active interaction.
    Idle,
    /// Orbiting camera (drag on empty space).
    OrbitCamera,
    /// Dragging to create a bond from an atom.
    DragBond { from_atom: usize },
    /// Dragging an atom to move it.
    DragAtom { atom_idx: usize },
}

/// Result of processing a pointer event.
#[derive(Debug, Clone)]
pub enum InteractionResult {
    /// No action needed.
    None,
    /// Spawn a new atom at this 3D position.
    SpawnAtom { position: Vec3 },
    /// Create a bond between two atoms.
    CreateBond { from: usize, to: usize },
    /// Select an atom for info display.
    SelectAtom { atom_idx: usize },
    /// Camera was orbited.
    CameraOrbited,
}

/// Manages user interaction state.
pub struct InteractionSystem {
    /// Current interaction mode.
    mode: InteractionMode,
    /// Whether pointer is currently pressed.
    pointer_down: bool,
    /// Position where pointer was pressed.
    pointer_start: Vec2,
    /// Last pointer position (for delta calculation).
    last_pointer: Vec2,
    /// Current pointer position.
    current_pointer: Vec2,
    /// Threshold for distinguishing click from drag.
    drag_threshold: f32,
}

impl InteractionSystem {
    pub fn new() -> Self {
        Self {
            mode: InteractionMode::Idle,
            pointer_down: false,
            pointer_start: Vec2::ZERO,
            last_pointer: Vec2::ZERO,
            current_pointer: Vec2::ZERO,
            drag_threshold: 8.0,
        }
    }

    /// Get the current interaction mode.
    pub fn mode(&self) -> InteractionMode {
        self.mode
    }

    /// Get the current pointer position.
    pub fn pointer_pos(&self) -> Vec2 {
        self.current_pointer
    }

    /// Handle pointer down event.
    pub fn on_pointer_down(
        &mut self,
        pos: Vec2,
        molecule: &MoleculeState3D,
        camera: &Camera3D,
        registry: &ElementRegistry,
    ) -> InteractionResult {
        self.pointer_down = true;
        self.pointer_start = pos;
        self.last_pointer = pos;
        self.current_pointer = pos;

        // Hit test for existing atoms
        if let Some(atom_idx) = hit_test_2d(molecule, camera, registry, pos) {
            // Clicked on an atom - start bond drag
            self.mode = InteractionMode::DragBond { from_atom: atom_idx };
            InteractionResult::SelectAtom { atom_idx }
        } else {
            // Clicked on empty space - might become orbit or spawn
            self.mode = InteractionMode::Idle;
            InteractionResult::None
        }
    }

    /// Handle pointer move event.
    pub fn on_pointer_move(
        &mut self,
        pos: Vec2,
        camera: &mut Camera3D,
    ) -> InteractionResult {
        self.current_pointer = pos;

        if !self.pointer_down {
            return InteractionResult::None;
        }

        let delta = pos - self.last_pointer;
        let total_moved = (pos - self.pointer_start).length();

        let result = match self.mode {
            InteractionMode::Idle => {
                // If moved enough, start camera orbit
                if total_moved > self.drag_threshold {
                    self.mode = InteractionMode::OrbitCamera;
                    camera.orbit(delta.x, delta.y);
                    InteractionResult::CameraOrbited
                } else {
                    InteractionResult::None
                }
            }
            InteractionMode::OrbitCamera => {
                camera.orbit(delta.x, delta.y);
                InteractionResult::CameraOrbited
            }
            InteractionMode::DragBond { .. } => {
                // Visual feedback handled elsewhere
                InteractionResult::None
            }
            InteractionMode::DragAtom { atom_idx: _ } => {
                // Atom dragging would update position here
                InteractionResult::None
            }
        };

        self.last_pointer = pos;
        result
    }

    /// Handle pointer up event.
    pub fn on_pointer_up(
        &mut self,
        pos: Vec2,
        molecule: &MoleculeState3D,
        camera: &Camera3D,
        registry: &ElementRegistry,
    ) -> InteractionResult {
        let total_moved = (pos - self.pointer_start).length();
        self.current_pointer = pos;

        let result = match self.mode {
            InteractionMode::Idle => {
                // Was a click (not drag) - spawn atom
                if total_moved < self.drag_threshold {
                    let world_pos = screen_to_world_on_plane(camera, pos, 0.0);
                    InteractionResult::SpawnAtom { position: world_pos }
                } else {
                    InteractionResult::None
                }
            }
            InteractionMode::OrbitCamera => {
                // Camera orbit ended
                InteractionResult::None
            }
            InteractionMode::DragBond { from_atom } => {
                // Check if dropped on another atom
                if let Some(to_atom) = hit_test_2d(molecule, camera, registry, pos) {
                    if to_atom != from_atom {
                        InteractionResult::CreateBond {
                            from: from_atom,
                            to: to_atom,
                        }
                    } else {
                        InteractionResult::None
                    }
                } else {
                    InteractionResult::None
                }
            }
            InteractionMode::DragAtom { .. } => {
                InteractionResult::None
            }
        };

        self.pointer_down = false;
        self.mode = InteractionMode::Idle;
        result
    }

    /// Reset the interaction state.
    pub fn reset(&mut self) {
        self.mode = InteractionMode::Idle;
        self.pointer_down = false;
    }

    /// Check if currently dragging for a bond.
    pub fn is_dragging_bond(&self) -> bool {
        matches!(self.mode, InteractionMode::DragBond { .. })
    }

    /// Get the source atom if dragging a bond.
    pub fn bond_drag_source(&self) -> Option<usize> {
        match self.mode {
            InteractionMode::DragBond { from_atom } => Some(from_atom),
            _ => None,
        }
    }
}

impl Default for InteractionSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_idle() {
        let system = InteractionSystem::new();
        assert_eq!(system.mode(), InteractionMode::Idle);
    }
}
