// Visibility mask post-process pipeline.
// Fullscreen pass that multiplies scene by a visibility mask texture.

import visibilityShaderSource from '../../visibility.wgsl?raw';

export interface VisibilityPipelineResult {
  pipeline: GPURenderPipeline;
  bindGroupLayout: GPUBindGroupLayout;
  nearestSampler: GPUSampler;
  linearSampler: GPUSampler;
  /** VisibilityUniforms (2 × vec4<f32> = 32 bytes), written per draw from camera.ts visibilityUniform. */
  uniformBuffer: GPUBuffer;
}

/**
 * Create the visibility mask post-process pipeline.
 * Bind group 0: scene texture, scene sampler, visibility texture, visibility sampler,
 * visibility uniforms (the camera's view and the game size; ADR-040).
 */
export function createVisibilityPipeline(
  device: GPUDevice,
  format: GPUTextureFormat,
): VisibilityPipelineResult {
  const shaderModule = device.createShaderModule({ code: visibilityShaderSource });

  const bindGroupLayout = device.createBindGroupLayout({
    entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
      { binding: 2, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
      { binding: 3, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
      { binding: 4, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
    ],
  });

  const pipelineLayout = device.createPipelineLayout({
    bindGroupLayouts: [bindGroupLayout],
  });

  const pipeline = device.createRenderPipeline({
    layout: pipelineLayout,
    vertex: {
      module: shaderModule,
      entryPoint: 'vs_visibility',
    },
    fragment: {
      module: shaderModule,
      entryPoint: 'fs_visibility',
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

  const nearestSampler = device.createSampler({
    magFilter: 'nearest',
    minFilter: 'nearest',
  });

  const linearSampler = device.createSampler({
    magFilter: 'linear',
    minFilter: 'linear',
  });

  const uniformBuffer = device.createBuffer({
    size: 32,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  return { pipeline, bindGroupLayout, nearestSampler, linearSampler, uniformBuffer };
}

/**
 * Create or update the visibility mask GPU texture from raw byte data.
 */
export function createVisibilityTexture(
  device: GPUDevice,
  cols: number,
  rows: number,
): GPUTexture {
  return device.createTexture({
    size: { width: cols, height: rows },
    format: 'r8unorm',
    usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
  });
}

/**
 * Upload visibility byte data to the GPU texture.
 */
export function uploadVisibilityData(
  device: GPUDevice,
  texture: GPUTexture,
  data: Uint8Array,
  cols: number,
  rows: number,
): void {
  device.queue.writeTexture(
    { texture },
    data,
    { bytesPerRow: cols },
    { width: cols, height: rows },
  );
}

/**
 * Create a bind group for the visibility post-process pass.
 */
export function createVisibilityBindGroup(
  device: GPUDevice,
  layout: GPUBindGroupLayout,
  sceneTextureView: GPUTextureView,
  sceneSampler: GPUSampler,
  visTextureView: GPUTextureView,
  visSampler: GPUSampler,
  uniformBuffer: GPUBuffer,
): GPUBindGroup {
  return device.createBindGroup({
    layout,
    entries: [
      { binding: 0, resource: sceneTextureView },
      { binding: 1, resource: sceneSampler },
      { binding: 2, resource: visTextureView },
      { binding: 3, resource: visSampler },
      { binding: 4, resource: { buffer: uniformBuffer } },
    ],
  });
}
