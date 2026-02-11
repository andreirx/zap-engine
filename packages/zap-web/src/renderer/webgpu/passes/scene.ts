// Scene render pass — main sprite/vector/SDF/effects rendering.

import type { LayerBatchDescriptor, BakeState } from '../../types';
import { LayerCompositor } from '../../compositor';
import { INSTANCE_STRIDE_BYTES, EFFECTS_VERTEX_BYTES, SDF_INSTANCE_STRIDE_BYTES, VECTOR_VERTEX_BYTES } from '../resources';

/** Function signature for drawing batch instances. */
export type DrawBatchFn = (
  pass: GPURenderPassEncoder,
  batchStart: number,
  batchEnd: number,
  batchAtlasSplit: number,
) => void;

export interface ScenePassConfig {
  // Pipelines
  alphaPipelines: GPURenderPipeline[];
  normalPipelines: GPURenderPipeline[];
  vectorPipeline: GPURenderPipeline;
  sdfPipeline: GPURenderPipeline;
  additivePipeline: GPURenderPipeline;
  // Bind groups
  cameraBindGroup: GPUBindGroup;
  textureBindGroups: GPUBindGroup[];
  normalTextureBindGroups: GPUBindGroup[];
  instanceBindGroup: GPUBindGroup;
  sdfBindGroup: GPUBindGroup;
  colorsBindGroup: GPUBindGroup;
  emptyBindGroup: GPUBindGroup;
  fallbackTextureBindGroup: GPUBindGroup;
  // Buffers
  effectsBuffer: GPUBuffer;
  vectorBuffer: GPUBuffer;
  // Compositor
  compositor: LayerCompositor;
}

/**
 * Create a function that draws batch instances using the correct atlas pipeline.
 */
export function createDrawBatchFn(config: ScenePassConfig): DrawBatchFn {
  const { alphaPipelines, cameraBindGroup, textureBindGroups, instanceBindGroup } = config;

  return (pass, batchStart, batchEnd, batchAtlasSplit) => {
    // Atlas 0 portion: [batchStart..batchAtlasSplit)
    const atlas0Count = batchAtlasSplit - batchStart;
    if (atlas0Count > 0 && alphaPipelines.length > 0) {
      pass.setPipeline(alphaPipelines[0]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, textureBindGroups[0]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas0Count, 0, batchStart);
    }

    // Atlas 1+ portion: [batchAtlasSplit..batchEnd)
    const atlas1Count = batchEnd - batchAtlasSplit;
    if (atlas1Count > 0 && alphaPipelines.length > 1) {
      pass.setPipeline(alphaPipelines[1]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, textureBindGroups[1]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas1Count, 0, batchAtlasSplit);
    } else if (atlas1Count > 0 && alphaPipelines.length === 1) {
      // Single atlas: draw remaining with same pipeline
      pass.setPipeline(alphaPipelines[0]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, textureBindGroups[0]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas1Count, 0, batchAtlasSplit);
    }
  };
}

/**
 * Create a function that draws batch instances using normal-map pipelines.
 */
export function createDrawNormalBatchFn(config: ScenePassConfig): DrawBatchFn {
  const { normalPipelines, cameraBindGroup, normalTextureBindGroups, instanceBindGroup } = config;

  return (pass, batchStart, batchEnd, batchAtlasSplit) => {
    const atlas0Count = batchAtlasSplit - batchStart;
    if (atlas0Count > 0 && normalPipelines.length > 0) {
      pass.setPipeline(normalPipelines[0]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, normalTextureBindGroups[0]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas0Count, 0, batchStart);
    }

    const atlas1Count = batchEnd - batchAtlasSplit;
    if (atlas1Count > 0 && normalPipelines.length > 1) {
      pass.setPipeline(normalPipelines[1]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, normalTextureBindGroups[1]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas1Count, 0, batchAtlasSplit);
    } else if (atlas1Count > 0 && normalPipelines.length === 1) {
      pass.setPipeline(normalPipelines[0]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, normalTextureBindGroups[0]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas1Count, 0, batchAtlasSplit);
    }
  };
}

/**
 * Encode the main scene render pass.
 */
export function encodeScenePass(
  pass: GPURenderPassEncoder,
  config: ScenePassConfig,
  drawBatchInstances: DrawBatchFn,
  instanceCount: number,
  atlasSplit: number,
  layerBatches: LayerBatchDescriptor[] | undefined,
  bakeState: BakeState | undefined,
  effectsVertexCount: number,
  sdfInstanceCount: number,
  vectorVertexCount: number,
): void {
  const {
    vectorPipeline,
    sdfPipeline,
    additivePipeline,
    cameraBindGroup,
    textureBindGroups,
    sdfBindGroup,
    colorsBindGroup,
    emptyBindGroup,
    fallbackTextureBindGroup,
    effectsBuffer,
    vectorBuffer,
    compositor,
  } = config;

  const hasBaking = bakeState && bakeState.bakedMask !== 0 && layerBatches && layerBatches.length > 0;

  // Draw sprite instances — layered with baking support
  if (layerBatches && layerBatches.length > 0) {
    for (const batch of layerBatches) {
      if (hasBaking && LayerCompositor.isLayerBaked(bakeState!.bakedMask, batch.layerId)) {
        // Blit cached texture for this layer
        const bindGroup = compositor.getBindGroup(batch.layerId);
        if (bindGroup) {
          pass.setPipeline(compositor.getPipeline());
          pass.setBindGroup(0, bindGroup);
          pass.draw(3); // Fullscreen triangle
        }
      } else {
        // Live layer: render instances directly
        drawBatchInstances(pass, batch.start, batch.end, batch.atlasSplit);
      }
    }
  } else {
    // Legacy path: single atlas split (no baking possible without layer batches)
    drawBatchInstances(pass, 0, instanceCount, atlasSplit);
  }

  // Vector geometry (alpha blend, drawn between sprites and SDF)
  if (vectorVertexCount > 0) {
    pass.setPipeline(vectorPipeline);
    pass.setBindGroup(0, cameraBindGroup);
    pass.setVertexBuffer(0, vectorBuffer);
    pass.draw(vectorVertexCount);
  }

  // SDF molecules (alpha blend, drawn between vectors and effects)
  if (sdfInstanceCount > 0) {
    pass.setPipeline(sdfPipeline);
    pass.setBindGroup(0, cameraBindGroup);
    pass.setBindGroup(1, sdfBindGroup);
    pass.draw(6, sdfInstanceCount);
  }

  // Effects (additive blend)
  if (effectsVertexCount > 0) {
    pass.setPipeline(additivePipeline);
    pass.setBindGroup(0, cameraBindGroup);
    pass.setBindGroup(1, textureBindGroups[0] ?? fallbackTextureBindGroup);
    pass.setBindGroup(2, emptyBindGroup);
    pass.setBindGroup(3, colorsBindGroup);
    pass.setVertexBuffer(0, effectsBuffer);
    pass.draw(effectsVertexCount);
  }
}

export interface NormalPassConfig {
  sdfNormalPipeline?: GPURenderPipeline;
  cameraBindGroup: GPUBindGroup;
  sdfBindGroup: GPUBindGroup;
}

/**
 * Encode the normal buffer render pass (when lighting + normal maps active).
 */
export function encodeNormalPass(
  pass: GPURenderPassEncoder,
  drawNormalBatchInstances: DrawBatchFn,
  instanceCount: number,
  atlasSplit: number,
  layerBatches: LayerBatchDescriptor[] | undefined,
  sdfInstanceCount: number,
  config?: NormalPassConfig,
): void {
  // Draw sprite normals
  if (layerBatches && layerBatches.length > 0) {
    for (const batch of layerBatches) {
      // Skip baked layers for normal pass (baked layer normals not cached yet)
      drawNormalBatchInstances(pass, batch.start, batch.end, batch.atlasSplit);
    }
  } else {
    drawNormalBatchInstances(pass, 0, instanceCount, atlasSplit);
  }

  // Draw SDF flat normals (prevents sprite normal bleeding onto SDF shapes)
  if (sdfInstanceCount > 0 && config?.sdfNormalPipeline) {
    pass.setPipeline(config.sdfNormalPipeline);
    pass.setBindGroup(0, config.cameraBindGroup);
    pass.setBindGroup(1, config.sdfBindGroup);
    pass.draw(6, sdfInstanceCount);
  }
}

/**
 * Encode the lighting post-process pass (scratch → screen).
 */
export function encodeLightingPass(
  pass: GPURenderPassEncoder,
  lightingPipeline: GPURenderPipeline,
  lightingBindGroup: GPUBindGroup,
): void {
  pass.setPipeline(lightingPipeline);
  pass.setBindGroup(0, lightingBindGroup);
  pass.draw(3); // Fullscreen triangle
}
