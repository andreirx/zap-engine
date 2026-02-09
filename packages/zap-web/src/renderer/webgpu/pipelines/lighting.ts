// Lighting post-process pipeline (fullscreen pass with point lights + normal maps).

import lightingShaderSource from '../../lighting.wgsl?raw';
import type { BufferResources } from '../resources';

export const LIGHT_FLOATS = 8;
export const MAX_LIGHTS_GPU = 64;

export interface LightingPipelineConfig {
  device: GPUDevice;
  format: GPUTextureFormat;
}

export interface LightingResources {
  pipeline: GPURenderPipeline;
  bindGroupLayout: GPUBindGroupLayout;
  sampler: GPUSampler;
}

/**
 * Create the lighting bind group layout.
 */
export function createLightingBindGroupLayout(device: GPUDevice): GPUBindGroupLayout {
  return device.createBindGroupLayout({
    entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
      { binding: 2, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
      { binding: 3, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'read-only-storage' } },
      { binding: 4, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
    ],
  });
}

/**
 * Create the lighting post-process pipeline.
 */
export function createLightingPipeline(config: LightingPipelineConfig): LightingResources {
  const { device, format } = config;

  const shaderModule = device.createShaderModule({ code: lightingShaderSource });
  const bindGroupLayout = createLightingBindGroupLayout(device);

  const pipeline = device.createRenderPipeline({
    layout: device.createPipelineLayout({ bindGroupLayouts: [bindGroupLayout] }),
    vertex: {
      module: shaderModule,
      entryPoint: 'vs_lighting',
    },
    fragment: {
      module: shaderModule,
      entryPoint: 'fs_lighting',
      targets: [{ format }],
    },
    primitive: { topology: 'triangle-list' },
  });

  const sampler = device.createSampler({
    magFilter: 'linear',
    minFilter: 'linear',
  });

  return { pipeline, bindGroupLayout, sampler };
}

/**
 * Create or recreate the lighting bind group when scratch texture changes.
 */
export function createLightingBindGroup(
  device: GPUDevice,
  layout: GPUBindGroupLayout,
  scratchView: GPUTextureView,
  normalBufferView: GPUTextureView,
  sampler: GPUSampler,
  buffers: BufferResources,
): GPUBindGroup {
  return device.createBindGroup({
    layout,
    entries: [
      { binding: 0, resource: scratchView },
      { binding: 1, resource: sampler },
      { binding: 2, resource: { buffer: buffers.lightUniformBuffer } },
      { binding: 3, resource: { buffer: buffers.lightStorageBuffer } },
      { binding: 4, resource: normalBufferView },
    ],
  });
}
