// LayerCompositor — manages intermediate render targets for baked layers.
//
// When a layer is marked as "baked", its sprite instances are rendered to an
// intermediate texture once and cached. Subsequent frames blit the cached
// texture instead of re-rendering all instances, saving GPU work for static
// layers (e.g., terrain with hundreds of tiles).
//
// Dirty detection uses a monotonic bake_generation counter encoded in the SAB
// header alongside the baked_layers bitmask.

import compositeShaderSource from './composite.wgsl?raw';

/** Per-layer cache entry. */
interface LayerCache {
  texture: GPUTexture;
  view: GPUTextureView;
  bindGroup: GPUBindGroup;
  /** The bake generation when this cache was last rendered. */
  lastBakeGen: number;
}

export class LayerCompositor {
  private device: GPUDevice;
  private format: GPUTextureFormat;
  private caches: Map<number, LayerCache> = new Map();
  private pipeline: GPURenderPipeline;
  private bindGroupLayout: GPUBindGroupLayout;
  private sampler: GPUSampler;
  private width: number;
  private height: number;

  constructor(
    device: GPUDevice,
    format: GPUTextureFormat,
    width: number,
    height: number,
  ) {
    this.device = device;
    this.format = format;
    this.width = width;
    this.height = height;

    const shaderModule = device.createShaderModule({ code: compositeShaderSource });

    this.sampler = device.createSampler({
      magFilter: 'linear',
      minFilter: 'linear',
    });

    this.bindGroupLayout = device.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
      ],
    });

    const pipelineLayout = device.createPipelineLayout({
      bindGroupLayouts: [this.bindGroupLayout],
    });

    this.pipeline = device.createRenderPipeline({
      layout: pipelineLayout,
      vertex: {
        module: shaderModule,
        entryPoint: 'vs_composite',
      },
      fragment: {
        module: shaderModule,
        entryPoint: 'fs_composite',
        targets: [{
          format,
          blend: {
            color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
            alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
          },
        }],
      },
      primitive: { topology: 'triangle-list' },
    });
  }

  /**
   * Decode the bake state from SAB header[21].
   * Returns { bakedMask, bakeGen }.
   */
  static decodeBakeState(encoded: number): { bakedMask: number; bakeGen: number } {
    const raw = Math.floor(encoded);
    return {
      bakedMask: raw & 0x3F,
      bakeGen: raw >>> 6,
    };
  }

  /** Check if a layer is baked according to the mask. */
  static isLayerBaked(bakedMask: number, layerId: number): boolean {
    return (bakedMask & (1 << layerId)) !== 0;
  }

  /**
   * Check if a layer's cache needs refreshing.
   * Returns true if the layer should be re-rendered to its intermediate texture.
   */
  needsRefresh(layerId: number, bakeGen: number): boolean {
    const cache = this.caches.get(layerId);
    if (!cache) return true;
    return cache.lastBakeGen !== bakeGen;
  }

  /**
   * Get or create the intermediate render target for a layer.
   * Returns the texture view suitable for use as a render pass color attachment.
   */
  getOrCreateTarget(layerId: number): { texture: GPUTexture; view: GPUTextureView } {
    let cache = this.caches.get(layerId);
    if (cache && cache.texture.width === this.width && cache.texture.height === this.height) {
      return { texture: cache.texture, view: cache.view };
    }

    // Destroy old texture if size changed
    if (cache) {
      cache.texture.destroy();
    }

    const texture = this.device.createTexture({
      size: { width: this.width, height: this.height },
      format: this.format,
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
    });
    const view = texture.createView();
    const bindGroup = this.device.createBindGroup({
      layout: this.bindGroupLayout,
      entries: [
        { binding: 0, resource: view },
        { binding: 1, resource: this.sampler },
      ],
    });

    const newCache: LayerCache = { texture, view, bindGroup, lastBakeGen: -1 };
    this.caches.set(layerId, newCache);
    return { texture, view };
  }

  /**
   * Mark a layer's cache as up-to-date with the given bake generation.
   * Call this after rendering the layer to its intermediate texture.
   */
  markClean(layerId: number, bakeGen: number): void {
    const cache = this.caches.get(layerId);
    if (cache) {
      cache.lastBakeGen = bakeGen;
    }
  }

  /**
   * Get the bind group for compositing a cached layer onto the screen.
   * Returns null if the layer has no cached texture.
   */
  getBindGroup(layerId: number): GPUBindGroup | null {
    return this.caches.get(layerId)?.bindGroup ?? null;
  }

  /** Get the composite pipeline for blitting cached textures. */
  getPipeline(): GPURenderPipeline {
    return this.pipeline;
  }

  /** Handle canvas resize — invalidates all cached textures. */
  resize(width: number, height: number): void {
    this.width = width;
    this.height = height;
    // Destroy all cached textures — they'll be recreated at the new size on next use
    for (const cache of this.caches.values()) {
      cache.texture.destroy();
    }
    this.caches.clear();
  }

  /** Remove a layer's cache (e.g., when it's no longer baked). */
  removeCache(layerId: number): void {
    const cache = this.caches.get(layerId);
    if (cache) {
      cache.texture.destroy();
      this.caches.delete(layerId);
    }
  }

  /** Destroy all resources. */
  destroy(): void {
    for (const cache of this.caches.values()) {
      cache.texture.destroy();
    }
    this.caches.clear();
  }
}
