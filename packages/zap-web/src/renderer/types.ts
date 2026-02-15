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
  /** Atlas split within this batch: instances [start..atlasSplit) use atlas 0. */
  atlasSplit: number;
}

/** Bake state decoded from SAB header — controls layer caching. */
export interface BakeState {
  /** Bitmask of which layers are baked (bits 0-5 = Background..UI). */
  bakedMask: number;
  /** Monotonic generation counter — changes signal cache invalidation. */
  bakeGen: number;
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
  ) => DrawTiming;

  /** Handle canvas resize. */
  resize: (width: number, height: number) => void;
}
