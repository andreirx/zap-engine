// Layer baking pass — renders baked layers to intermediate textures.

import type { LayerBatchDescriptor, BakeState } from '../../types';
import { LayerCompositor } from '../../compositor';
import type { DrawBatchFn, DrawBlendBatchFn } from './scene';

/**
 * Render baked+dirty layers to intermediate textures.
 * Groups all batches per layer so that multiple (blend, atlas) batches
 * within the same baked layer are all rendered into the cache.
 * Returns true if any layers were baked.
 */
export function encodeBakePass(
  encoder: GPUCommandEncoder,
  compositor: LayerCompositor,
  layerBatches: LayerBatchDescriptor[],
  bakeState: BakeState,
  drawBatchInstances: DrawBatchFn,
  drawBlendBatchInstances?: DrawBlendBatchFn,
): boolean {
  let anyBaked = false;

  // Collect which layers are baked and dirty, then render all batches for each
  const dirtyLayers = new Set<number>();
  for (const batch of layerBatches) {
    if (LayerCompositor.isLayerBaked(bakeState.bakedMask, batch.layerId)
        && compositor.needsRefresh(batch.layerId, bakeState.bakeGen)) {
      dirtyLayers.add(batch.layerId);
    }
  }

  for (const layerId of dirtyLayers) {
    const { view: targetView } = compositor.getOrCreateTarget(layerId);
    const layerPass = encoder.beginRenderPass({
      colorAttachments: [{
        view: targetView,
        clearValue: { r: 0, g: 0, b: 0, a: 0 },
        loadOp: 'clear',
        storeOp: 'store',
      }],
    });

    // Draw ALL batches belonging to this layer (blend-aware if available)
    for (const batch of layerBatches) {
      if (batch.layerId === layerId) {
        if (drawBlendBatchInstances && batch.blendMode !== undefined) {
          drawBlendBatchInstances(layerPass, batch.start, batch.end, batch.atlasId, batch.blendMode);
        } else {
          drawBatchInstances(layerPass, batch.start, batch.end, batch.atlasId);
        }
      }
    }

    layerPass.end();
    compositor.markClean(layerId, bakeState.bakeGen);
    anyBaked = true;
  }

  return anyBaked;
}
