// Common pipeline layouts and blend configurations.

import type { LayoutResources } from '../resources';

/**
 * Create pipeline layout for sprite rendering (camera + texture + instances).
 */
export function createSpritePipelineLayout(
  device: GPUDevice,
  layouts: LayoutResources,
): GPUPipelineLayout {
  return device.createPipelineLayout({
    bindGroupLayouts: [
      layouts.cameraBindGroupLayout,
      layouts.textureBindGroupLayout,
      layouts.instanceBindGroupLayout,
    ],
  });
}

/**
 * Create pipeline layout for effects rendering (camera + texture + empty + colors).
 */
export function createEffectsPipelineLayout(
  device: GPUDevice,
  layouts: LayoutResources,
): GPUPipelineLayout {
  return device.createPipelineLayout({
    bindGroupLayouts: [
      layouts.cameraBindGroupLayout,
      layouts.textureBindGroupLayout,
      layouts.emptyBindGroupLayout,
      layouts.colorsBindGroupLayout,
    ],
  });
}

/**
 * Create pipeline layout for SDF rendering (camera + sdf storage + lights).
 */
export function createSdfPipelineLayout(
  device: GPUDevice,
  layouts: LayoutResources,
): GPUPipelineLayout {
  return device.createPipelineLayout({
    bindGroupLayouts: [
      layouts.cameraBindGroupLayout,
      layouts.sdfBindGroupLayout,
      layouts.sdfLightBindGroupLayout,
    ],
  });
}

/**
 * Create pipeline layout for vector rendering (camera only).
 */
export function createVectorPipelineLayout(
  device: GPUDevice,
  layouts: LayoutResources,
): GPUPipelineLayout {
  return device.createPipelineLayout({
    bindGroupLayouts: [layouts.cameraBindGroupLayout],
  });
}

/**
 * Alpha blend targets for standard sprite rendering.
 */
export function alphaBlendTargets(format: GPUTextureFormat): GPUColorTargetState[] {
  return [{
    format,
    blend: {
      color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
      alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
    },
  }];
}

/**
 * Additive blend targets for effects/glow rendering.
 */
export function additiveBlendTargets(format: GPUTextureFormat): GPUColorTargetState[] {
  return [{
    format,
    blend: {
      color: { srcFactor: 'src-alpha', dstFactor: 'one', operation: 'add' },
      alpha: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
    },
  }];
}

/**
 * Normal map blend targets (always rgba8unorm).
 */
export function normalBlendTargets(): GPUColorTargetState[] {
  return [{
    format: 'rgba8unorm',
    blend: {
      color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
      alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
    },
  }];
}
