// SDF molecule (raymarched shapes) render pipeline.

import sdfShaderSource from '../../molecule.wgsl?raw';
import { alphaBlendTargets } from './common';

export interface SdfPipelineConfig {
  device: GPUDevice;
  layout: GPUPipelineLayout;
  format: GPUTextureFormat;
  emissiveMult: number;
}

/**
 * Create shader module for SDF rendering.
 */
export function createSdfShaderModule(device: GPUDevice): GPUShaderModule {
  return device.createShaderModule({ code: sdfShaderSource });
}

/**
 * Create the SDF molecule pipeline for raymarched spheres, capsules, and boxes.
 */
export function createSdfPipeline(config: SdfPipelineConfig): GPURenderPipeline {
  const { device, layout, format, emissiveMult } = config;

  const shaderModule = createSdfShaderModule(device);

  return device.createRenderPipeline({
    layout,
    vertex: {
      module: shaderModule,
      entryPoint: 'vs_sdf',
    },
    fragment: {
      module: shaderModule,
      entryPoint: 'fs_sdf',
      constants: { SDF_EMISSIVE_MULT: emissiveMult },
      targets: alphaBlendTargets(format),
    },
    primitive: { topology: 'triangle-list' },
  });
}
