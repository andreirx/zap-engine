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
  HEADER_ALPHA_EFFECTS_VERTEX_COUNT,
  HEADER_ALPHA_EFFECTS_BATCH_COUNT,
  INSTANCE_FLOATS,
  EFFECTS_VERTEX_FLOATS,
  SDF_INSTANCE_FLOATS,
  VECTOR_VERTEX_FLOATS,
  LAYER_BATCH_FLOATS,
  LIGHT_FLOATS,
  ALPHA_EFFECTS_BATCH_FLOATS,
  HEADER_VISIBILITY_COLS,
  HEADER_VISIBILITY_ROWS,
  HEADER_VISIBILITY_INTERPOLATION,
} from './protocol';
import type { LayerBatchDescriptor, AlphaEffectsBatchDescriptor, BakeState, LightingState, VisibilityState } from '../renderer/types';

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
  /** Alpha effects vertex data (smoke/dust particles). */
  alphaEffectsData?: Float32Array;
  /** Number of alpha effects vertices. */
  alphaEffectsVertexCount: number;
  /** Alpha effects batch descriptors (one per layer with alpha particles). */
  alphaEffectsBatches?: AlphaEffectsBatchDescriptor[];
  /** Visibility mask state. */
  visibilityState?: VisibilityState;
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

  const alphaEffectsVertexCount = buf[HEADER_ALPHA_EFFECTS_VERTEX_COUNT] ?? 0;
  const visCols = buf[HEADER_VISIBILITY_COLS] ?? 0;
  const visRows = buf[HEADER_VISIBILITY_ROWS] ?? 0;

  // Nothing to render
  if (instanceCount === 0 && sdfInstanceCount === 0 && vectorVertexCount === 0
      && effectsVertexCount === 0 && alphaEffectsVertexCount === 0
      && (visCols === 0 || visRows === 0)) {
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
        atlasId: buf[base + 3],
        blendMode: buf[base + 4] ?? 0,
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

  // Alpha effects data
  const alphaEffectsBatchCount = buf[HEADER_ALPHA_EFFECTS_BATCH_COUNT] ?? 0;
  let alphaEffectsData: Float32Array | undefined;
  let alphaEffectsBatches: AlphaEffectsBatchDescriptor[] | undefined;

  if (alphaEffectsVertexCount > 0) {
    alphaEffectsData = buf.subarray(
      layout.alphaEffectsDataOffset,
      layout.alphaEffectsDataOffset + alphaEffectsVertexCount * EFFECTS_VERTEX_FLOATS,
    );
  }

  if (alphaEffectsBatchCount > 0) {
    alphaEffectsBatches = [];
    for (let i = 0; i < alphaEffectsBatchCount; i++) {
      const base = layout.alphaEffectsBatchDataOffset + i * ALPHA_EFFECTS_BATCH_FLOATS;
      alphaEffectsBatches.push({
        layerId: buf[base],
        startVertex: buf[base + 1],
        endVertex: buf[base + 2],
      });
    }
  }

  // Visibility mask data
  let visibilityState: VisibilityState | undefined;
  if (visCols > 0 && visRows > 0) {
    const byteCount = visCols * visRows;
    // Read raw bytes from SAB at the computed byte offset
    const sharedU8 = new Uint8Array(buf.buffer, buf.byteOffset);
    const visData = sharedU8.subarray(
      layout.visibilityDataByteOffset,
      layout.visibilityDataByteOffset + byteCount,
    );
    visibilityState = {
      cols: visCols,
      rows: visRows,
      data: visData,
      interpolation: buf[HEADER_VISIBILITY_INTERPOLATION] ?? 0,
    };
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
    alphaEffectsData,
    alphaEffectsVertexCount,
    alphaEffectsBatches,
    visibilityState,
    wasmTimeUs,
  };
}
