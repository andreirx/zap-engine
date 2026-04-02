// Scene render pass — main sprite/vector/SDF/effects rendering.

import type { LayerBatchDescriptor, AlphaEffectsBatchDescriptor, BakeState } from '../../types';
import { LayerCompositor } from '../../compositor';
import { INSTANCE_STRIDE_BYTES, EFFECTS_VERTEX_BYTES, SDF_INSTANCE_STRIDE_BYTES, VECTOR_VERTEX_BYTES } from '../resources';

/** Function signature for drawing batch instances. */
export type DrawBatchFn = (
  pass: GPURenderPassEncoder,
  batchStart: number,
  batchEnd: number,
  batchAtlasId: number,
) => void;

export interface ScenePassConfig {
  // Pipelines
  alphaPipelines: GPURenderPipeline[];
  additiveSpritePipelines: GPURenderPipeline[];
  normalPipelines: GPURenderPipeline[];
  vectorPipeline: GPURenderPipeline;
  sdfPipeline: GPURenderPipeline;
  additivePipeline: GPURenderPipeline;
  alphaEffectsPipeline?: GPURenderPipeline;
  // Bind groups
  cameraBindGroup: GPUBindGroup;
  textureBindGroups: GPUBindGroup[];
  normalTextureBindGroups: GPUBindGroup[];
  instanceBindGroup: GPUBindGroup;
  sdfBindGroup: GPUBindGroup;
  sdfLightBindGroup: GPUBindGroup;
  colorsBindGroup: GPUBindGroup;
  emptyBindGroup: GPUBindGroup;
  fallbackTextureBindGroup: GPUBindGroup;
  // Buffers
  effectsBuffer: GPUBuffer;
  alphaEffectsBuffer?: GPUBuffer;
  vectorBuffer: GPUBuffer;
  // Compositor
  compositor: LayerCompositor;
}

/** Function signature for drawing batch instances with blend mode awareness. */
export type DrawBlendBatchFn = (
  pass: GPURenderPassEncoder,
  batchStart: number,
  batchEnd: number,
  batchAtlasId: number,
  blendMode: number,
) => void;

/**
 * Create a function that draws batch instances using the correct atlas pipeline.
 * With N-atlas support, each batch has a single atlasId and all instances use that atlas.
 * Blend mode selects between alpha (0) and additive (1) sprite pipelines.
 */
export function createDrawBatchFn(config: ScenePassConfig): DrawBatchFn {
  const { alphaPipelines, cameraBindGroup, textureBindGroups, instanceBindGroup } = config;

  return (pass, batchStart, batchEnd, batchAtlasId) => {
    const instanceCount = batchEnd - batchStart;
    if (instanceCount <= 0) return;

    // Clamp atlas ID to available pipelines (fallback to atlas 0 if out of range)
    const atlasIdx = Math.min(batchAtlasId, alphaPipelines.length - 1);
    if (atlasIdx < 0 || alphaPipelines.length === 0) return;

    pass.setPipeline(alphaPipelines[atlasIdx]);
    pass.setBindGroup(0, cameraBindGroup);
    pass.setBindGroup(1, textureBindGroups[atlasIdx]);
    pass.setBindGroup(2, instanceBindGroup);
    pass.draw(6, instanceCount, 0, batchStart);
  };
}

/**
 * Create a blend-mode-aware function that selects alpha or additive sprite pipeline.
 */
export function createDrawBlendBatchFn(config: ScenePassConfig): DrawBlendBatchFn {
  const { alphaPipelines, additiveSpritePipelines, cameraBindGroup, textureBindGroups, instanceBindGroup } = config;

  return (pass, batchStart, batchEnd, batchAtlasId, blendMode) => {
    const instanceCount = batchEnd - batchStart;
    if (instanceCount <= 0) return;

    const pipelines = blendMode === 1 ? additiveSpritePipelines : alphaPipelines;

    // Clamp atlas ID to available pipelines (fallback to atlas 0 if out of range)
    const atlasIdx = Math.min(batchAtlasId, pipelines.length - 1);
    if (atlasIdx < 0 || pipelines.length === 0) return;

    pass.setPipeline(pipelines[atlasIdx]);
    pass.setBindGroup(0, cameraBindGroup);
    pass.setBindGroup(1, textureBindGroups[atlasIdx]);
    pass.setBindGroup(2, instanceBindGroup);
    pass.draw(6, instanceCount, 0, batchStart);
  };
}

/**
 * Create a function that draws batch instances using normal-map pipelines.
 * With N-atlas support, each batch has a single atlasId and all instances use that atlas.
 */
export function createDrawNormalBatchFn(config: ScenePassConfig): DrawBatchFn {
  const { normalPipelines, cameraBindGroup, normalTextureBindGroups, instanceBindGroup } = config;

  return (pass, batchStart, batchEnd, batchAtlasId) => {
    const instanceCount = batchEnd - batchStart;
    if (instanceCount <= 0) return;

    // Clamp atlas ID to available pipelines (fallback to atlas 0 if out of range)
    const atlasIdx = Math.min(batchAtlasId, normalPipelines.length - 1);
    if (atlasIdx < 0 || normalPipelines.length === 0) return;

    pass.setPipeline(normalPipelines[atlasIdx]);
    pass.setBindGroup(0, cameraBindGroup);
    pass.setBindGroup(1, normalTextureBindGroups[atlasIdx]);
    pass.setBindGroup(2, instanceBindGroup);
    pass.draw(6, instanceCount, 0, batchStart);
  };
}

/**
 * Encode the main scene render pass.
 * When drawBlendBatchInstances is provided, uses blend-mode-aware pipeline selection.
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
  drawBlendBatchInstances?: DrawBlendBatchFn,
  alphaEffectsVertexCount?: number,
  alphaEffectsBatches?: AlphaEffectsBatchDescriptor[],
  /** Optional layer range filter [minLayer, maxLayer] inclusive. Default: all layers. */
  layerRange?: [number, number],
): void {
  const {
    vectorPipeline,
    sdfPipeline,
    additivePipeline,
    alphaEffectsPipeline,
    cameraBindGroup,
    textureBindGroups,
    sdfBindGroup,
    sdfLightBindGroup,
    colorsBindGroup,
    emptyBindGroup,
    fallbackTextureBindGroup,
    effectsBuffer,
    alphaEffectsBuffer,
    vectorBuffer,
    compositor,
  } = config;

  const hasBaking = bakeState && bakeState.bakedMask !== 0 && layerBatches && layerBatches.length > 0;
  const hasAlphaEffects = alphaEffectsPipeline && alphaEffectsBuffer
    && alphaEffectsVertexCount && alphaEffectsVertexCount > 0
    && alphaEffectsBatches && alphaEffectsBatches.length > 0;
  const minLayer = layerRange ? layerRange[0] : -Infinity;
  const maxLayer = layerRange ? layerRange[1] : Infinity;

  // Helper: draw alpha effects for a given layer (if any)
  function drawAlphaEffectsForLayer(layerId: number) {
    if (!hasAlphaEffects) return;
    for (const aeBatch of alphaEffectsBatches!) {
      if (aeBatch.layerId === layerId) {
        const vertCount = aeBatch.endVertex - aeBatch.startVertex;
        if (vertCount <= 0) continue;
        pass.setPipeline(alphaEffectsPipeline!);
        pass.setBindGroup(0, cameraBindGroup);
        pass.setBindGroup(1, textureBindGroups[0] ?? fallbackTextureBindGroup);
        pass.setBindGroup(2, emptyBindGroup);
        pass.setBindGroup(3, colorsBindGroup);
        pass.setVertexBuffer(0, alphaEffectsBuffer!);
        pass.draw(vertCount, 1, aeBatch.startVertex);
      }
    }
  }

  // Draw sprite instances — layered with baking, blend mode, and alpha effects support
  const drawnAlphaLayers = new Set<number>();

  if (layerBatches && layerBatches.length > 0) {
    let lastLayerId = -1;

    for (const batch of layerBatches) {
      // Skip batches outside the layer range
      if (batch.layerId < minLayer || batch.layerId > maxLayer) continue;

      // When transitioning to a new layer, draw alpha effects for the previous layer
      if (batch.layerId !== lastLayerId && lastLayerId >= 0) {
        drawAlphaEffectsForLayer(lastLayerId);
        drawnAlphaLayers.add(lastLayerId);
      }
      lastLayerId = batch.layerId;

      if (hasBaking && LayerCompositor.isLayerBaked(bakeState!.bakedMask, batch.layerId)) {
        // Blit cached texture for this layer
        const bindGroup = compositor.getBindGroup(batch.layerId);
        if (bindGroup) {
          pass.setPipeline(compositor.getPipeline());
          pass.setBindGroup(0, bindGroup);
          pass.draw(3); // Fullscreen triangle
        }
      } else if (drawBlendBatchInstances && batch.blendMode !== undefined) {
        // Blend-aware path: select alpha or additive sprite pipeline
        drawBlendBatchInstances(pass, batch.start, batch.end, batch.atlasId, batch.blendMode);
      } else {
        // Legacy path: always alpha blend
        drawBatchInstances(pass, batch.start, batch.end, batch.atlasId);
      }
    }

    // Draw alpha effects for the final sprite layer
    if (lastLayerId >= 0) {
      drawAlphaEffectsForLayer(lastLayerId);
      drawnAlphaLayers.add(lastLayerId);
    }
  } else {
    // Legacy path: atlasSplit marks where atlas 0 ends, atlas 1 begins
    // Draw atlas 0 portion [0..atlasSplit)
    if (atlasSplit > 0) {
      drawBatchInstances(pass, 0, atlasSplit, 0);
    }
    // Draw atlas 1 portion [atlasSplit..instanceCount)
    if (atlasSplit < instanceCount) {
      drawBatchInstances(pass, atlasSplit, instanceCount, 1);
    }
  }

  // Draw alpha effects for layers that had no sprite batches (e.g., standalone smoke on VFX layer)
  if (hasAlphaEffects) {
    for (const aeBatch of alphaEffectsBatches!) {
      if (aeBatch.layerId < minLayer || aeBatch.layerId > maxLayer) continue;
      if (!drawnAlphaLayers.has(aeBatch.layerId)) {
        drawAlphaEffectsForLayer(aeBatch.layerId);
        drawnAlphaLayers.add(aeBatch.layerId);
      }
    }
  }

  // Vectors, SDF, and additive effects are world content — skip when rendering UI-only
  const isWorldPass = !layerRange || minLayer < 5;

  // Vector geometry (alpha blend, drawn between sprites and SDF)
  if (isWorldPass && vectorVertexCount > 0) {
    pass.setPipeline(vectorPipeline);
    pass.setBindGroup(0, cameraBindGroup);
    pass.setVertexBuffer(0, vectorBuffer);
    pass.draw(vectorVertexCount);
  }

  // SDF molecules (alpha blend, drawn between vectors and effects)
  if (isWorldPass && sdfInstanceCount > 0) {
    pass.setPipeline(sdfPipeline);
    pass.setBindGroup(0, cameraBindGroup);
    pass.setBindGroup(1, sdfBindGroup);
    pass.setBindGroup(2, sdfLightBindGroup);
    pass.draw(6, sdfInstanceCount);
  }

  // Effects (additive blend)
  if (isWorldPass && effectsVertexCount > 0) {
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
  sdfLightBindGroup: GPUBindGroup;  // Required even for normal pass (pipeline layout expects it)
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
  /** Optional layer range filter [minLayer, maxLayer] inclusive. Default: all layers. */
  layerRange?: [number, number],
): void {
  const minL = layerRange ? layerRange[0] : -Infinity;
  const maxL = layerRange ? layerRange[1] : Infinity;

  // Draw sprite normals
  if (layerBatches && layerBatches.length > 0) {
    for (const batch of layerBatches) {
      if (batch.layerId < minL || batch.layerId > maxL) continue;
      drawNormalBatchInstances(pass, batch.start, batch.end, batch.atlasId);
    }
  } else {
    // Legacy path: atlasSplit marks where atlas 0 ends, atlas 1 begins
    if (atlasSplit > 0) {
      drawNormalBatchInstances(pass, 0, atlasSplit, 0);
    }
    if (atlasSplit < instanceCount) {
      drawNormalBatchInstances(pass, atlasSplit, instanceCount, 1);
    }
  }

  // Draw SDF flat normals (prevents sprite normal bleeding onto SDF shapes)
  if (sdfInstanceCount > 0 && config?.sdfNormalPipeline) {
    pass.setPipeline(config.sdfNormalPipeline);
    pass.setBindGroup(0, config.cameraBindGroup);
    pass.setBindGroup(1, config.sdfBindGroup);
    pass.setBindGroup(2, config.sdfLightBindGroup);  // Pipeline layout requires 3 groups
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
