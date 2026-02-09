// Additive effects (particles, arcs) render pipeline.

import shaderSource from '../../shaders.wgsl?raw';
import { EFFECTS_VERTEX_BYTES } from '../resources';
import { additiveBlendTargets } from './common';

export interface EffectsPipelineConfig {
  device: GPUDevice;
  layout: GPUPipelineLayout;
  format: GPUTextureFormat;
  glowMult: number;
}

/**
 * Create the additive effects pipeline for particles and electric arcs.
 */
export function createEffectsPipeline(config: EffectsPipelineConfig): GPURenderPipeline {
  const { device, layout, format, glowMult } = config;

  // Reuse the sprite shader module (contains vs_effects and fs_additive entry points)
  const shaderModule = device.createShaderModule({ code: shaderSource });

  return device.createRenderPipeline({
    layout,
    vertex: {
      module: shaderModule,
      entryPoint: 'vs_effects',
      buffers: [{
        arrayStride: EFFECTS_VERTEX_BYTES,
        attributes: [
          { shaderLocation: 0, offset: 0, format: 'float32x3' },
          { shaderLocation: 1, offset: 12, format: 'float32x2' },
        ],
      }],
    },
    fragment: {
      module: shaderModule,
      entryPoint: 'fs_additive',
      constants: { EFFECTS_HDR_MULT: glowMult },
      targets: additiveBlendTargets(format),
    },
    primitive: { topology: 'triangle-list' },
  });
}
