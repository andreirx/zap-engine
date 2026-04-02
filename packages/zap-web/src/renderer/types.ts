// Common Renderer interface — implemented by both WebGPU and Canvas 2D backends.

/** Render tier describes the negotiated surface capability. */
export type RenderTier = 'hdr-edr' | 'hdr-srgb' | 'sdr' | 'canvas2d';

/** Layer batch descriptor from the SAB. Mirrors Rust LayerBatch. */
export interface LayerBatchDescriptor {
  /** RenderLayer enum value (0=Background, 1=Terrain, 2=Objects, etc.). */
  layerId: number;
  /** Start index (inclusive) in the instance buffer. */
  start: number;
  /** End index (exclusive) in the instance buffer. */
  end: number;
  /** Atlas ID for this batch (index into manifest's atlas list). All instances in batch use this atlas. */
  atlasId: number;
  /** Blend mode: 0 = Alpha (standard), 1 = Additive (glow). */
  blendMode: number;
}

/** Bake state decoded from SAB header — controls layer caching. */
export interface BakeState {
  /** Bitmask of which layers are baked (bits 0-5 = Background..UI). */
  bakedMask: number;
  /** Monotonic generation counter — changes signal cache invalidation. */
  bakeGen: number;
}

/** Alpha effects batch descriptor — one per layer that has alpha particles. */
export interface AlphaEffectsBatchDescriptor {
  /** RenderLayer enum value. */
  layerId: number;
  /** Start vertex index (inclusive) in the alpha effects buffer. */
  startVertex: number;
  /** End vertex index (exclusive) in the alpha effects buffer. */
  endVertex: number;
}

/** Lighting state decoded from SAB header + light data section. */
export interface LightingState {
  /** Flat f32 array of point lights (8 floats each: x, y, r, g, b, intensity, radius, layer_mask). */
  lightData: Float32Array;
  /** Number of active lights. */
  lightCount: number;
  /** Ambient light RGB. */
  ambient: [number, number, number];
}

/** Visibility mask state decoded from SAB. */
export interface VisibilityState {
  /** Grid width in cells. */
  cols: number;
  /** Grid height in cells. */
  rows: number;
  /** Raw visibility bytes (0=hidden, 255=visible). */
  data: Uint8Array;
  /** Interpolation: 0=nearest, 1=linear. */
  interpolation: number;
}

/** Timing data returned from renderer.draw() */
export interface DrawTiming {
  /** Time spent submitting draw commands (μs). */
  drawUs: number;
  /** Time spent in actual rasterization/GPU work (μs). Canvas2D: measured via getImageData sync. */
  rasterUs: number;
}

export interface Renderer {
  /** The active backend: 'webgpu' for HDR/EDR, 'canvas2d' for fallback. */
  backend: 'webgpu' | 'canvas2d';

  /** The negotiated render tier (HDR capability level). */
  tier: RenderTier;

  /**
   * Draw one frame.
   * @param instanceData  Flat float array of sprites (8 floats each: x, y, rot, scale, sprite_col, alpha, cell_span, atlas_row)
   * @param instanceCount Total sprite instances
   * @param atlasSplit    Legacy: how many use atlas 0 (used when no layer batches)
   * @param effectsData   Optional flat float array of effect vertices (5 floats each: x, y, z, u, v)
   * @param effectsVertexCount Total effect vertices
   * @param sdfData       Optional flat float array of SDF instances (12 floats each)
   * @param sdfInstanceCount Total SDF instances
   * @param vectorData    Optional flat float array of vector vertices (6 floats each: x, y, r, g, b, a)
   * @param vectorVertexCount Total vector vertices
   * @param layerBatches  Optional layer batch descriptors for layered rendering
   * @param bakeState     Optional bake state for layer caching
   * @param lightingState Optional dynamic lighting data (point lights + ambient)
   * @returns Timing data for profiling (draw submission + rasterization times)
   */
  draw: (
    instanceData: Float32Array,
    instanceCount: number,
    atlasSplit: number,
    effectsData?: Float32Array,
    effectsVertexCount?: number,
    sdfData?: Float32Array,
    sdfInstanceCount?: number,
    vectorData?: Float32Array,
    vectorVertexCount?: number,
    layerBatches?: LayerBatchDescriptor[],
    bakeState?: BakeState,
    lightingState?: LightingState,
    alphaEffectsData?: Float32Array,
    alphaEffectsVertexCount?: number,
    alphaEffectsBatches?: AlphaEffectsBatchDescriptor[],
    visibilityState?: VisibilityState,
  ) => DrawTiming;

  /** Handle canvas resize. */
  resize: (width: number, height: number) => void;
}
