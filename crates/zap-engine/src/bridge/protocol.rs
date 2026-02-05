/// SharedArrayBuffer layout constants.
/// Must stay in sync with TypeScript `protocol.ts`.
///
/// Layout (all values in f32 / 4 bytes):
/// ```text
/// [Header: 12 floats]
/// [Instances: MAX_INSTANCES × 8 floats]
/// [Effects: MAX_EFFECTS_VERTICES × 5 floats]
/// [Sounds: MAX_SOUNDS × 1 u8 (packed into f32 slots)]
/// [Events: MAX_EVENTS × 4 floats]
/// ```

/// Number of floats in the header section.
pub const HEADER_FLOATS: usize = 12;

/// Header field indices.
pub const HEADER_LOCK: usize = 0;
pub const HEADER_FRAME_COUNTER: usize = 1;
pub const HEADER_INSTANCE_COUNT: usize = 2;
pub const HEADER_ATLAS_SPLIT: usize = 3;
pub const HEADER_EFFECTS_VERTEX_COUNT: usize = 4;
pub const HEADER_WORLD_WIDTH: usize = 5;
pub const HEADER_WORLD_HEIGHT: usize = 6;
pub const HEADER_SOUND_COUNT: usize = 7;
pub const HEADER_EVENT_COUNT: usize = 8;
pub const HEADER_RESERVED_0: usize = 9;
pub const HEADER_RESERVED_1: usize = 10;
pub const HEADER_RESERVED_2: usize = 11;

/// Maximum number of render instances.
pub const MAX_INSTANCES: usize = 512;

/// Floats per render instance.
pub const INSTANCE_FLOATS: usize = 8;

/// Maximum number of effects vertices.
pub const MAX_EFFECTS_VERTICES: usize = 16384;

/// Floats per effects vertex (x, y, z, u, v).
pub const EFFECTS_VERTEX_FLOATS: usize = 5;

/// Maximum number of sound events per frame.
pub const MAX_SOUNDS: usize = 32;

/// Maximum number of game events per frame.
pub const MAX_EVENTS: usize = 32;

/// Floats per game event (kind, a, b, c).
pub const EVENT_FLOATS: usize = 4;

/// Total size of instance data section in floats.
pub const INSTANCE_DATA_FLOATS: usize = MAX_INSTANCES * INSTANCE_FLOATS;

/// Total size of effects data section in floats.
pub const EFFECTS_DATA_FLOATS: usize = MAX_EFFECTS_VERTICES * EFFECTS_VERTEX_FLOATS;

/// Total size of sound data section in floats (1 float per sound for simplicity).
pub const SOUND_DATA_FLOATS: usize = MAX_SOUNDS;

/// Total size of events data section in floats.
pub const EVENT_DATA_FLOATS: usize = MAX_EVENTS * EVENT_FLOATS;

/// Byte offset where instance data begins.
pub const INSTANCE_DATA_OFFSET: usize = HEADER_FLOATS;

/// Byte offset where effects data begins.
pub const EFFECTS_DATA_OFFSET: usize = INSTANCE_DATA_OFFSET + INSTANCE_DATA_FLOATS;

/// Byte offset where sound data begins.
pub const SOUND_DATA_OFFSET: usize = EFFECTS_DATA_OFFSET + EFFECTS_DATA_FLOATS;

/// Byte offset where event data begins.
pub const EVENT_DATA_OFFSET: usize = SOUND_DATA_OFFSET + SOUND_DATA_FLOATS;

/// Total buffer size in floats.
pub const BUFFER_TOTAL_FLOATS: usize =
    HEADER_FLOATS + INSTANCE_DATA_FLOATS + EFFECTS_DATA_FLOATS + SOUND_DATA_FLOATS + EVENT_DATA_FLOATS;

/// Total buffer size in bytes.
pub const BUFFER_TOTAL_BYTES: usize = BUFFER_TOTAL_FLOATS * 4;
