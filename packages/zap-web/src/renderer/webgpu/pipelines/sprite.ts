// Sprite and normal-map render pipelines.

import shaderSource from '../../shaders.wgsl?raw';
import type { AssetManifest } from '../../../assets/manifest';
import { alphaBlendTargets, normalBlendTargets } from './common';

export interface SpritePipelineConfig {
  device: GPUDevice;
  layout: GPUPipelineLayout;
  format: GPUTextureFormat;
  manifest: AssetManifest;
}

/**
 * Create shader module for sprite rendering.
 */
export function createSpriteShaderModule(device: GPUDevice): GPUShaderModule {
  return device.createShaderModule({ code: shaderSource });
}

/**
 * Create one alpha-blend pipeline per atlas for sprite rendering.
 */
export function createSpritePipelines(
  config: SpritePipelineConfig,
  shaderModule: GPUShaderModule,
): GPURenderPipeline[] {
  const { device, layout, format, manifest } = config;

  return manifest.atlases.map((atlas) =>
    device.createRenderPipeline({
      layout,
      vertex: {
        module: shaderModule,
        entryPoint: 'vs_main',
        constants: { ATLAS_COLS: atlas.cols, ATLAS_ROWS: atlas.rows },
      },
      fragment: {
        module: shaderModule,
        entryPoint: 'fs_main',
        targets: alphaBlendTargets(format),
      },
      primitive: { topology: 'triangle-list' },
    })
  );
}

/**
 * Create one normal-map pipeline per atlas (renders to rgba8unorm normal buffer).
 */
export function createNormalPipelines(
  config: SpritePipelineConfig,
  shaderModule: GPUShaderModule,
): GPURenderPipeline[] {
  const { device, layout, manifest } = config;

  return manifest.atlases.map((atlas) =>
    device.createRenderPipeline({
      layout,
      vertex: {
        module: shaderModule,
        entryPoint: 'vs_main',
        constants: { ATLAS_COLS: atlas.cols, ATLAS_ROWS: atlas.rows },
      },
      fragment: {
        module: shaderModule,
        entryPoint: 'fs_normal',
        targets: normalBlendTargets(),
      },
      primitive: { topology: 'triangle-list' },
    })
  );
}
