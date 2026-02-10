// frame-reader.ts — Extracts frame state from SharedArrayBuffer.
//
// Decodes: instances, effects, SDF, vectors, layer batches, bake state, lighting.
// Used by React hook and can be used by non-React consumers.

import type { ProtocolLayout } from './protocol';
import {
  HEADER_INSTANCE_COUNT,
  HEADER_ATLAS_SPLIT,
  HEADER_EFFECTS_VERTEX_COUNT,
  HEADER_SDF_INSTANCE_COUNT,
  HEADER_VECTOR_VERTEX_COUNT,
  HEADER_LAYER_BATCH_COUNT,
  HEADER_BAKE_STATE,
  HEADER_LIGHT_COUNT,
  HEADER_AMBIENT_R,
  HEADER_AMBIENT_G,
  HEADER_AMBIENT_B,
  HEADER_WASM_TIME_US,
  INSTANCE_FLOATS,
  EFFECTS_VERTEX_FLOATS,
  SDF_INSTANCE_FLOATS,
  VECTOR_VERTEX_FLOATS,
  LAYER_BATCH_FLOATS,
  LIGHT_FLOATS,
} from './protocol';
import type { LayerBatchDescriptor, BakeState, LightingState } from '../renderer/types';

/** Complete frame state extracted from SharedArrayBuffer. */
export interface FrameState {
  /** Render instance data (positions, sprites, etc.). */
  instanceData: Float32Array;
  /** Number of render instances. */
  instanceCount: number;
  /** Atlas split point (first N instances use atlas 0). */
  atlasSplit: number;
  /** Effects vertex data (electric arcs, particles). */
  effectsData?: Float32Array;
  /** Number of effects vertices. */
  effectsVertexCount: number;
  /** SDF instance data (raymarched shapes). */
  sdfData?: Float32Array;
  /** Number of SDF instances. */
  sdfInstanceCount: number;
  /** Vector vertex data (polygons, lines). */
  vectorData?: Float32Array;
  /** Number of vector vertices. */
  vectorVertexCount: number;
  /** Layer batch descriptors for render ordering. */
  layerBatches?: LayerBatchDescriptor[];
  /** Layer baking state for render caching. */
  bakeState?: BakeState;
  /** Dynamic lighting state. */
  lightingState?: LightingState;
  /** WASM tick execution time in microseconds. */
  wasmTimeUs: number;
}

/**
 * Read frame state from SharedArrayBuffer.
 *
 * Returns null if there's nothing to render (no instances, SDF, or vectors).
 * The subarrays returned are views into the original buffer — zero-copy.
 */
export function readFrameState(buf: Float32Array, layout: ProtocolLayout): FrameState | null {
  const instanceCount = buf[HEADER_INSTANCE_COUNT];
  const atlasSplit = buf[HEADER_ATLAS_SPLIT];
  const effectsVertexCount = buf[HEADER_EFFECTS_VERTEX_COUNT];
  const sdfInstanceCount = buf[HEADER_SDF_INSTANCE_COUNT];
  const vectorVertexCount = buf[HEADER_VECTOR_VERTEX_COUNT];
  const layerBatchCount = buf[HEADER_LAYER_BATCH_COUNT] ?? 0;

  // Nothing to render
  if (instanceCount === 0 && sdfInstanceCount === 0 && vectorVertexCount === 0) {
    return null;
  }

  // Instance data
  const instanceData = buf.subarray(
    layout.instanceDataOffset,
    layout.instanceDataOffset + instanceCount * INSTANCE_FLOATS,
  );

  // Effects data
  let effectsData: Float32Array | undefined;
  if (effectsVertexCount > 0) {
    effectsData = buf.subarray(
      layout.effectsDataOffset,
      layout.effectsDataOffset + effectsVertexCount * EFFECTS_VERTEX_FLOATS,
    );
  }

  // SDF data
  let sdfData: Float32Array | undefined;
  if (sdfInstanceCount > 0) {
    sdfData = buf.subarray(
      layout.sdfDataOffset,
      layout.sdfDataOffset + sdfInstanceCount * SDF_INSTANCE_FLOATS,
    );
  }

  // Vector data
  let vectorData: Float32Array | undefined;
  if (vectorVertexCount > 0) {
    vectorData = buf.subarray(
      layout.vectorDataOffset,
      layout.vectorDataOffset + vectorVertexCount * VECTOR_VERTEX_FLOATS,
    );
  }

  // Layer batches
  let layerBatches: LayerBatchDescriptor[] | undefined;
  if (layerBatchCount > 0) {
    layerBatches = [];
    for (let i = 0; i < layerBatchCount; i++) {
      const base = layout.layerBatchDataOffset + i * LAYER_BATCH_FLOATS;
      layerBatches.push({
        layerId: buf[base],
        start: buf[base + 1],
        end: buf[base + 2],
        atlasSplit: buf[base + 3],
      });
    }
  }

  // Bake state
  let bakeState: BakeState | undefined;
  const rawBakeState = buf[HEADER_BAKE_STATE];
  if (rawBakeState > 0) {
    const raw = Math.floor(rawBakeState);
    bakeState = {
      bakedMask: raw & 0x3F,
      bakeGen: raw >>> 6,
    };
  }

  // Lighting state
  let lightingState: LightingState | undefined;
  const lightCount = buf[HEADER_LIGHT_COUNT] ?? 0;
  if (lightCount > 0) {
    lightingState = {
      lightData: buf.subarray(
        layout.lightDataOffset,
        layout.lightDataOffset + lightCount * LIGHT_FLOATS,
      ),
      lightCount,
      ambient: [buf[HEADER_AMBIENT_R], buf[HEADER_AMBIENT_G], buf[HEADER_AMBIENT_B]],
    };
  } else {
    // Even with no lights, pass ambient if it's not default white
    const ar = buf[HEADER_AMBIENT_R] ?? 1.0;
    const ag = buf[HEADER_AMBIENT_G] ?? 1.0;
    const ab = buf[HEADER_AMBIENT_B] ?? 1.0;
    if (ar < 1.0 || ag < 1.0 || ab < 1.0) {
      lightingState = {
        lightData: new Float32Array(0),
        lightCount: 0,
        ambient: [ar, ag, ab],
      };
    }
  }

  // Read WASM timing
  const wasmTimeUs = buf[HEADER_WASM_TIME_US] ?? 0;

  return {
    instanceData,
    instanceCount,
    atlasSplit,
    effectsData,
    effectsVertexCount,
    sdfData,
    sdfInstanceCount,
    vectorData,
    vectorVertexCount,
    layerBatches,
    bakeState,
    lightingState,
    wasmTimeUs,
  };
}
