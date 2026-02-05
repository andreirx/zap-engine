use bytemuck::{Pod, Zeroable};

/// Unique identifier for an entity in the scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

/// A sound event emitted by the game logic.
/// The numeric value maps to a game-defined sound in the TypeScript SoundManager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SoundEvent(pub u32);

/// A game event communicated from Rust to TypeScript via SharedArrayBuffer.
/// Generic container: `kind` identifies the event, `a/b/c` carry payload.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct GameEvent {
    pub kind: f32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
}

impl GameEvent {
    pub const FLOATS: usize = 4;
}
