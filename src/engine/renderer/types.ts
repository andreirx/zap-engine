// Common Renderer interface — implemented by both WebGPU and Canvas 2D backends.

/** Render tier describes the negotiated surface capability. */
export type RenderTier = 'hdr-edr' | 'hdr-srgb' | 'sdr' | 'canvas2d';

export interface Renderer {
  /** The active backend: 'webgpu' for HDR/EDR, 'canvas2d' for fallback. */
  backend: 'webgpu' | 'canvas2d';

  /** The negotiated render tier (HDR capability level). */
  tier: RenderTier;

  /**
   * Draw one frame.
   * @param instanceData  Flat float array of sprites (8 floats each: x, y, rot, scale, sprite_col, alpha, cell_span, atlas_row)
   * @param instanceCount Total sprite instances
   * @param atlasSplit    How many use atlas 0 (alpha blend); rest use atlas 1+ (additive or second atlas)
   * @param effectsData   Optional flat float array of effect vertices (5 floats each: x, y, z, u, v)
   * @param effectsVertexCount Total effect vertices
   * @param sdfData       Optional flat float array of SDF instances (12 floats each)
   * @param sdfInstanceCount Total SDF instances
   * @param vectorData    Optional flat float array of vector vertices (6 floats each: x, y, r, g, b, a)
   * @param vectorVertexCount Total vector vertices
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
  ) => void;

  /** Handle canvas resize. */
  resize: (width: number, height: number) => void;
}
