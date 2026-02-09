// Vector geometry (Lyon-tessellated polygons) render pipeline.

import vectorShaderSource from '../../vector.wgsl?raw';
import { VECTOR_VERTEX_BYTES } from '../resources';
import { alphaBlendTargets } from './common';

export interface VectorPipelineConfig {
  device: GPUDevice;
  layout: GPUPipelineLayout;
  format: GPUTextureFormat;
  glowMult: number;
}

/**
 * Create shader module for vector rendering.
 */
export function createVectorShaderModule(device: GPUDevice): GPUShaderModule {
  return device.createShaderModule({ code: vectorShaderSource });
}

/**
 * Create the vector pipeline for CPU-tessellated polygons and lines.
 */
export function createVectorPipeline(config: VectorPipelineConfig): GPURenderPipeline {
  const { device, layout, format, glowMult } = config;

  const shaderModule = createVectorShaderModule(device);

  return device.createRenderPipeline({
    layout,
    vertex: {
      module: shaderModule,
      entryPoint: 'vs_vector',
      buffers: [{
        arrayStride: VECTOR_VERTEX_BYTES,
        attributes: [
          { shaderLocation: 0, offset: 0, format: 'float32x2' },   // position
          { shaderLocation: 1, offset: 8, format: 'float32x4' },   // color
        ],
      }],
    },
    fragment: {
      module: shaderModule,
      entryPoint: 'fs_vector',
      constants: { VECTOR_HDR_MULT: glowMult },
      targets: alphaBlendTargets(format),
    },
    primitive: { topology: 'triangle-list' },
  });
}
