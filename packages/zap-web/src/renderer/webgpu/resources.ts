// WebGPU resource management: textures, buffers, and bind groups.

import type { AssetManifest, GPUTextureAsset } from '../../assets/manifest';
import { createGPUTextureFromBlob } from '../../assets/loader';
import { packColorsForGPU } from '../constants';
import {
  INSTANCE_STRIDE_BYTES,
  EFFECTS_VERTEX_BYTES,
  SDF_INSTANCE_STRIDE_BYTES,
  VECTOR_VERTEX_BYTES,
} from '../../worker/protocol';

// Re-export for use in other modules
export { INSTANCE_STRIDE_BYTES, EFFECTS_VERTEX_BYTES, SDF_INSTANCE_STRIDE_BYTES, VECTOR_VERTEX_BYTES };

export interface TextureResources {
  textures: GPUTextureAsset[];
  normalTextures: (GPUTextureAsset | null)[];
  hasNormalMaps: boolean;
  sampler: GPUSampler;
  flatNormalView: GPUTextureView;
  fallbackTextureBindGroup: GPUBindGroup;
  textureBindGroups: GPUBindGroup[];
  normalTextureBindGroups: GPUBindGroup[];
}

export interface BufferResources {
  cameraBuffer: GPUBuffer;
  colorsBuffer: GPUBuffer;
  instanceBuffer: GPUBuffer;
  effectsBuffer: GPUBuffer;
  vectorBuffer: GPUBuffer;
  sdfStorageBuffer: GPUBuffer;
  lightUniformBuffer: GPUBuffer;
  lightStorageBuffer: GPUBuffer;
}

export interface BindGroupResources {
  cameraBindGroup: GPUBindGroup;
  colorsBindGroup: GPUBindGroup;
  instanceBindGroup: GPUBindGroup;
  sdfBindGroup: GPUBindGroup;
  sdfLightBindGroup: GPUBindGroup;
  emptyBindGroup: GPUBindGroup;
}

export interface LayoutResources {
  cameraBindGroupLayout: GPUBindGroupLayout;
  textureBindGroupLayout: GPUBindGroupLayout;
  instanceBindGroupLayout: GPUBindGroupLayout;
  colorsBindGroupLayout: GPUBindGroupLayout;
  sdfBindGroupLayout: GPUBindGroupLayout;
  sdfLightBindGroupLayout: GPUBindGroupLayout;
  emptyBindGroupLayout: GPUBindGroupLayout;
}

/**
 * Load atlas textures from blobs.
 */
export async function loadAtlasTextures(
  device: GPUDevice,
  manifest: AssetManifest,
  atlasBlobs: Map<string, Blob>,
): Promise<GPUTextureAsset[]> {
  const textures: GPUTextureAsset[] = [];
  for (const atlas of manifest.atlases) {
    const blob = atlasBlobs.get(atlas.name);
    if (!blob) {
      throw new Error(`Missing blob for atlas: ${atlas.name}`);
    }
    textures.push(await createGPUTextureFromBlob(device, blob));
  }
  return textures;
}

/**
 * Load normal map textures (optional, per-atlas).
 * Normal maps are loaded WITHOUT premultiplied alpha to preserve raw normal values.
 */
export async function loadNormalTextures(
  device: GPUDevice,
  manifest: AssetManifest,
  normalMapBlobs?: Map<string, Blob>,
): Promise<(GPUTextureAsset | null)[]> {
  const normalTextures: (GPUTextureAsset | null)[] = [];
  for (const atlas of manifest.atlases) {
    const normalBlob = normalMapBlobs?.get(atlas.name);
    if (normalBlob) {
      normalTextures.push(await createGPUTextureFromBlob(device, normalBlob, false));
    } else {
      normalTextures.push(null);
    }
  }
  return normalTextures;
}

/**
 * Create sampler for texture filtering.
 */
export function createSampler(device: GPUDevice): GPUSampler {
  return device.createSampler({
    magFilter: 'linear',
    minFilter: 'linear',
    mipmapFilter: 'linear',
  });
}

/**
 * Create flat normal placeholder texture (1x1 RGBA: 128,128,255,255 = tangent-space (0,0,1)).
 */
export function createFlatNormalTexture(device: GPUDevice): GPUTextureView {
  const flatNormalTexture = device.createTexture({
    size: { width: 1, height: 1 },
    format: 'rgba8unorm',
    usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
  });
  device.queue.writeTexture(
    { texture: flatNormalTexture },
    new Uint8Array([128, 128, 255, 255]),
    { bytesPerRow: 4 },
    { width: 1, height: 1 },
  );
  return flatNormalTexture.createView();
}

/**
 * Create fallback 1x1 white texture for effects when no atlases exist.
 */
export function createFallbackTexture(device: GPUDevice): GPUTexture {
  const fallbackTex = device.createTexture({
    size: [1, 1],
    format: 'rgba8unorm',
    usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
  });
  device.queue.writeTexture(
    { texture: fallbackTex },
    new Uint8Array([255, 255, 255, 255]),
    { bytesPerRow: 4 },
    [1, 1],
  );
  return fallbackTex;
}

/**
 * Create all bind group layouts.
 */
export function createLayouts(device: GPUDevice): LayoutResources {
  const cameraBindGroupLayout = device.createBindGroupLayout({
    entries: [{
      binding: 0,
      visibility: GPUShaderStage.VERTEX,
      buffer: { type: 'uniform' },
    }],
  });

  const textureBindGroupLayout = device.createBindGroupLayout({
    entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
    ],
  });

  const instanceBindGroupLayout = device.createBindGroupLayout({
    entries: [{
      binding: 0,
      visibility: GPUShaderStage.VERTEX,
      buffer: { type: 'read-only-storage' },
    }],
  });

  const colorsBindGroupLayout = device.createBindGroupLayout({
    entries: [{
      binding: 0,
      visibility: GPUShaderStage.FRAGMENT,
      buffer: { type: 'uniform' },
    }],
  });

  const sdfBindGroupLayout = device.createBindGroupLayout({
    entries: [{
      binding: 0,
      visibility: GPUShaderStage.VERTEX,
      buffer: { type: 'read-only-storage' },
    }],
  });

  // SDF light bind group: light uniforms + light storage for dynamic lighting
  const sdfLightBindGroupLayout = device.createBindGroupLayout({
    entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'read-only-storage' } },
    ],
  });

  const emptyBindGroupLayout = device.createBindGroupLayout({ entries: [] });

  return {
    cameraBindGroupLayout,
    textureBindGroupLayout,
    instanceBindGroupLayout,
    colorsBindGroupLayout,
    sdfBindGroupLayout,
    sdfLightBindGroupLayout,
    emptyBindGroupLayout,
  };
}

/**
 * Create all GPU buffers.
 */
export function createBuffers(
  device: GPUDevice,
  maxInstances: number,
  maxEffectsVertices: number,
  maxSdfInstances: number,
  maxVectorVertices: number,
  maxLights: number,
): BufferResources {
  const cameraBuffer = device.createBuffer({
    size: 64,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const colorsData = packColorsForGPU();
  const colorsBuffer = device.createBuffer({
    size: colorsData.byteLength,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(colorsBuffer, 0, colorsData);

  const instanceBuffer = device.createBuffer({
    size: INSTANCE_STRIDE_BYTES * maxInstances,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });

  const effectsBuffer = device.createBuffer({
    size: EFFECTS_VERTEX_BYTES * maxEffectsVertices,
    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  });

  const vectorBuffer = device.createBuffer({
    size: VECTOR_VERTEX_BYTES * maxVectorVertices,
    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  });

  const sdfStorageBuffer = device.createBuffer({
    size: SDF_INSTANCE_STRIDE_BYTES * maxSdfInstances,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });

  // Light uniform buffer: LightUniforms = 2 x vec4<f32> = 32 bytes
  const lightUniformBuffer = device.createBuffer({
    size: 32,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  // Light storage buffer: array<PointLight>, each 8 x f32 = 32 bytes
  const LIGHT_FLOATS = 8;
  const lightStorageBuffer = device.createBuffer({
    size: maxLights * LIGHT_FLOATS * 4,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });

  return {
    cameraBuffer,
    colorsBuffer,
    instanceBuffer,
    effectsBuffer,
    vectorBuffer,
    sdfStorageBuffer,
    lightUniformBuffer,
    lightStorageBuffer,
  };
}

/**
 * Create bind groups for buffers.
 */
export function createBindGroups(
  device: GPUDevice,
  layouts: LayoutResources,
  buffers: BufferResources,
): BindGroupResources {
  const cameraBindGroup = device.createBindGroup({
    layout: layouts.cameraBindGroupLayout,
    entries: [{ binding: 0, resource: { buffer: buffers.cameraBuffer } }],
  });

  const colorsBindGroup = device.createBindGroup({
    layout: layouts.colorsBindGroupLayout,
    entries: [{ binding: 0, resource: { buffer: buffers.colorsBuffer } }],
  });

  const instanceBindGroup = device.createBindGroup({
    layout: layouts.instanceBindGroupLayout,
    entries: [{ binding: 0, resource: { buffer: buffers.instanceBuffer } }],
  });

  const sdfBindGroup = device.createBindGroup({
    layout: layouts.sdfBindGroupLayout,
    entries: [{ binding: 0, resource: { buffer: buffers.sdfStorageBuffer } }],
  });

  const sdfLightBindGroup = device.createBindGroup({
    layout: layouts.sdfLightBindGroupLayout,
    entries: [
      { binding: 0, resource: { buffer: buffers.lightUniformBuffer } },
      { binding: 1, resource: { buffer: buffers.lightStorageBuffer } },
    ],
  });

  const emptyBindGroup = device.createBindGroup({
    layout: layouts.emptyBindGroupLayout,
    entries: [],
  });

  return {
    cameraBindGroup,
    colorsBindGroup,
    instanceBindGroup,
    sdfBindGroup,
    sdfLightBindGroup,
    emptyBindGroup,
  };
}

/**
 * Create texture bind groups for atlases.
 */
export function createTextureBindGroups(
  device: GPUDevice,
  layout: GPUBindGroupLayout,
  textures: GPUTextureAsset[],
  sampler: GPUSampler,
): GPUBindGroup[] {
  return textures.map((tex) =>
    device.createBindGroup({
      layout,
      entries: [
        { binding: 0, resource: tex.view },
        { binding: 1, resource: sampler },
      ],
    })
  );
}

/**
 * Create normal texture bind groups.
 */
export function createNormalTextureBindGroups(
  device: GPUDevice,
  layout: GPUBindGroupLayout,
  normalTextures: (GPUTextureAsset | null)[],
  flatNormalView: GPUTextureView,
  sampler: GPUSampler,
): GPUBindGroup[] {
  return normalTextures.map((nt) =>
    device.createBindGroup({
      layout,
      entries: [
        { binding: 0, resource: nt?.view ?? flatNormalView },
        { binding: 1, resource: sampler },
      ],
    })
  );
}
