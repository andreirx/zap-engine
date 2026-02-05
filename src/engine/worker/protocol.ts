// SharedArrayBuffer layout constants — mirrors Rust bridge/protocol.rs.
// MUST stay in sync with the Rust side.

/** Number of floats in the header section. */
export const HEADER_FLOATS = 12;

/** Header field indices. */
export const HEADER_LOCK = 0;
export const HEADER_FRAME_COUNTER = 1;
export const HEADER_INSTANCE_COUNT = 2;
export const HEADER_ATLAS_SPLIT = 3;
export const HEADER_EFFECTS_VERTEX_COUNT = 4;
export const HEADER_WORLD_WIDTH = 5;
export const HEADER_WORLD_HEIGHT = 6;
export const HEADER_SOUND_COUNT = 7;
export const HEADER_EVENT_COUNT = 8;

/** Maximum number of render instances. */
export const MAX_INSTANCES = 512;

/** Floats per render instance. */
export const INSTANCE_FLOATS = 8;

/** Maximum number of effects vertices. */
export const MAX_EFFECTS_VERTICES = 16384;

/** Floats per effects vertex (x, y, z, u, v). */
export const EFFECTS_VERTEX_FLOATS = 5;

/** Maximum number of sound events per frame. */
export const MAX_SOUNDS = 32;

/** Maximum number of game events per frame. */
export const MAX_EVENTS = 32;

/** Floats per game event (kind, a, b, c). */
export const EVENT_FLOATS = 4;

/** Total size of instance data section in floats. */
export const INSTANCE_DATA_FLOATS = MAX_INSTANCES * INSTANCE_FLOATS;

/** Total size of effects data section in floats. */
export const EFFECTS_DATA_FLOATS = MAX_EFFECTS_VERTICES * EFFECTS_VERTEX_FLOATS;

/** Total size of sound data section in floats. */
export const SOUND_DATA_FLOATS = MAX_SOUNDS;

/** Total size of events data section in floats. */
export const EVENT_DATA_FLOATS = MAX_EVENTS * EVENT_FLOATS;

/** Offset (in floats) where instance data begins. */
export const INSTANCE_DATA_OFFSET = HEADER_FLOATS;

/** Offset (in floats) where effects data begins. */
export const EFFECTS_DATA_OFFSET = INSTANCE_DATA_OFFSET + INSTANCE_DATA_FLOATS;

/** Offset (in floats) where sound data begins. */
export const SOUND_DATA_OFFSET = EFFECTS_DATA_OFFSET + EFFECTS_DATA_FLOATS;

/** Offset (in floats) where event data begins. */
export const EVENT_DATA_OFFSET = SOUND_DATA_OFFSET + SOUND_DATA_FLOATS;

/** Total buffer size in floats. */
export const BUFFER_TOTAL_FLOATS =
  HEADER_FLOATS + INSTANCE_DATA_FLOATS + EFFECTS_DATA_FLOATS + SOUND_DATA_FLOATS + EVENT_DATA_FLOATS;

/** Total buffer size in bytes. */
export const BUFFER_TOTAL_BYTES = BUFFER_TOTAL_FLOATS * 4;
