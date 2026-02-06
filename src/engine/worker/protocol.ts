// SharedArrayBuffer layout — mirrors Rust bridge/protocol.rs.
// MUST stay in sync with the Rust side.
//
// Capacities are written once into the header at init.
// TypeScript reads them from the header to compute offsets dynamically.

/** Number of floats in the header section. */
export const HEADER_FLOATS = 22;

/** Header field indices. */
export const HEADER_LOCK = 0;
export const HEADER_FRAME_COUNTER = 1;
export const HEADER_MAX_INSTANCES = 2;
export const HEADER_INSTANCE_COUNT = 3;
export const HEADER_ATLAS_SPLIT = 4;
export const HEADER_MAX_EFFECTS_VERTICES = 5;
export const HEADER_EFFECTS_VERTEX_COUNT = 6;
export const HEADER_WORLD_WIDTH = 7;
export const HEADER_WORLD_HEIGHT = 8;
export const HEADER_MAX_SOUNDS = 9;
export const HEADER_SOUND_COUNT = 10;
export const HEADER_MAX_EVENTS = 11;
export const HEADER_EVENT_COUNT = 12;
export const HEADER_PROTOCOL_VERSION = 13;
export const HEADER_MAX_SDF_INSTANCES = 14;
export const HEADER_SDF_INSTANCE_COUNT = 15;
export const HEADER_MAX_VECTOR_VERTICES = 16;
export const HEADER_VECTOR_VERTEX_COUNT = 17;
// Phase 8: Layer batches
export const HEADER_MAX_LAYER_BATCHES = 18;
export const HEADER_LAYER_BATCH_COUNT = 19;
export const HEADER_LAYER_BATCH_OFFSET = 20;
/** Encoded bake state: baked_layers_mask | (bake_generation << 6). */
export const HEADER_BAKE_STATE = 21;

/** Protocol version written into the header. */
export const PROTOCOL_VERSION = 3.0;

/** Floats per render instance (wire format — never changes). */
export const INSTANCE_FLOATS = 8;

/** Floats per effects vertex: x, y, z, u, v (wire format — never changes). */
export const EFFECTS_VERTEX_FLOATS = 5;

/** Floats per game event: kind, a, b, c (wire format — never changes). */
export const EVENT_FLOATS = 4;

/** Floats per SDF instance: x, y, radius, rotation, r, g, b, shininess, emissive, shape_type, half_height, extra. */
export const SDF_INSTANCE_FLOATS = 12;

/** Floats per vector vertex: x, y, r, g, b, a (wire format — never changes). */
export const VECTOR_VERTEX_FLOATS = 6;

/** Floats per layer batch descriptor: layer_id, start, end, atlas_split. */
export const LAYER_BATCH_FLOATS = 4;

/** Default maximum layer batches (one per RenderLayer). */
export const DEFAULT_MAX_LAYER_BATCHES = 6;

/**
 * Runtime-computed buffer layout. Replaces the old compile-time MAX_* constants.
 * Mirrors the Rust `ProtocolLayout` struct.
 */
export class ProtocolLayout {
  readonly maxInstances: number;
  readonly maxEffectsVertices: number;
  readonly maxSounds: number;
  readonly maxEvents: number;
  readonly maxSdfInstances: number;
  readonly maxVectorVertices: number;
  readonly maxLayerBatches: number;

  readonly instanceDataFloats: number;
  readonly effectsDataFloats: number;
  readonly soundDataFloats: number;
  readonly eventDataFloats: number;
  readonly sdfDataFloats: number;
  readonly vectorDataFloats: number;
  readonly layerBatchDataFloats: number;

  readonly instanceDataOffset: number;
  readonly effectsDataOffset: number;
  readonly soundDataOffset: number;
  readonly eventDataOffset: number;
  readonly sdfDataOffset: number;
  readonly vectorDataOffset: number;
  readonly layerBatchDataOffset: number;

  readonly bufferTotalFloats: number;
  readonly bufferTotalBytes: number;

  constructor(
    maxInstances: number,
    maxEffectsVertices: number,
    maxSounds: number,
    maxEvents: number,
    maxSdfInstances: number = 128,
    maxVectorVertices: number = 0,
    maxLayerBatches: number = DEFAULT_MAX_LAYER_BATCHES,
  ) {
    this.maxInstances = maxInstances;
    this.maxEffectsVertices = maxEffectsVertices;
    this.maxSounds = maxSounds;
    this.maxEvents = maxEvents;
    this.maxSdfInstances = maxSdfInstances;
    this.maxVectorVertices = maxVectorVertices;
    this.maxLayerBatches = maxLayerBatches;

    this.instanceDataFloats = maxInstances * INSTANCE_FLOATS;
    this.effectsDataFloats = maxEffectsVertices * EFFECTS_VERTEX_FLOATS;
    this.soundDataFloats = maxSounds;
    this.eventDataFloats = maxEvents * EVENT_FLOATS;
    this.sdfDataFloats = maxSdfInstances * SDF_INSTANCE_FLOATS;
    this.vectorDataFloats = maxVectorVertices * VECTOR_VERTEX_FLOATS;
    this.layerBatchDataFloats = maxLayerBatches * LAYER_BATCH_FLOATS;

    this.instanceDataOffset = HEADER_FLOATS;
    this.effectsDataOffset = this.instanceDataOffset + this.instanceDataFloats;
    this.soundDataOffset = this.effectsDataOffset + this.effectsDataFloats;
    this.eventDataOffset = this.soundDataOffset + this.soundDataFloats;
    this.sdfDataOffset = this.eventDataOffset + this.eventDataFloats;
    this.vectorDataOffset = this.sdfDataOffset + this.sdfDataFloats;
    this.layerBatchDataOffset = this.vectorDataOffset + this.vectorDataFloats;

    this.bufferTotalFloats = this.layerBatchDataOffset + this.layerBatchDataFloats;
    this.bufferTotalBytes = this.bufferTotalFloats * 4;
  }

  /** Read capacities from a SharedArrayBuffer header (written by the worker at init). */
  static fromHeader(f32: Float32Array): ProtocolLayout {
    return new ProtocolLayout(
      f32[HEADER_MAX_INSTANCES],
      f32[HEADER_MAX_EFFECTS_VERTICES],
      f32[HEADER_MAX_SOUNDS],
      f32[HEADER_MAX_EVENTS],
      f32[HEADER_MAX_SDF_INSTANCES],
      f32[HEADER_MAX_VECTOR_VERTICES],
      f32[HEADER_MAX_LAYER_BATCHES],
    );
  }

  /** Read capacities from WASM accessor functions (called in the worker). */
  static fromWasm(exports: {
    get_max_instances: () => number;
    get_max_effects_vertices: () => number;
    get_max_sounds: () => number;
    get_max_events: () => number;
    get_max_sdf_instances: () => number;
    get_max_vector_vertices?: () => number;
    get_max_layer_batches?: () => number;
  }): ProtocolLayout {
    return new ProtocolLayout(
      exports.get_max_instances(),
      exports.get_max_effects_vertices(),
      exports.get_max_sounds(),
      exports.get_max_events(),
      exports.get_max_sdf_instances(),
      exports.get_max_vector_vertices?.() ?? 0,
      exports.get_max_layer_batches?.() ?? DEFAULT_MAX_LAYER_BATCHES,
    );
  }
}
