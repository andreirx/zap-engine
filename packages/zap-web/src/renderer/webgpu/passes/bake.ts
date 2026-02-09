// Layer baking pass — renders baked layers to intermediate textures.

import type { LayerBatchDescriptor, BakeState } from '../../types';
import { LayerCompositor } from '../../compositor';
import type { DrawBatchFn } from './scene';

/**
 * Render baked+dirty layers to intermediate textures.
 * Returns true if any layers were baked.
 */
export function encodeBakePass(
  encoder: GPUCommandEncoder,
  compositor: LayerCompositor,
  layerBatches: LayerBatchDescriptor[],
  bakeState: BakeState,
  drawBatchInstances: DrawBatchFn,
): boolean {
  let anyBaked = false;

  for (const batch of layerBatches) {
    if (!LayerCompositor.isLayerBaked(bakeState.bakedMask, batch.layerId)) continue;
    if (!compositor.needsRefresh(batch.layerId, bakeState.bakeGen)) continue;

    // Render this layer's instances to an intermediate texture
    const { view: targetView } = compositor.getOrCreateTarget(batch.layerId);
    const layerPass = encoder.beginRenderPass({
      colorAttachments: [{
        view: targetView,
        clearValue: { r: 0, g: 0, b: 0, a: 0 },
        loadOp: 'clear',
        storeOp: 'store',
      }],
    });
    drawBatchInstances(layerPass, batch.start, batch.end, batch.atlasSplit);
    layerPass.end();

    compositor.markClean(batch.layerId, bakeState.bakeGen);
    anyBaked = true;
  }

  return anyBaked;
}
