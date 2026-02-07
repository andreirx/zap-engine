// ZapEngine Template — TypeScript entry point
// Initializes the engine worker, renderer, and connects them.

import {
  initRenderer,
  loadManifest,
  loadAssetBlobs,
  HEADER_INSTANCE_COUNT,
  HEADER_ATLAS_SPLIT,
  HEADER_EFFECTS_VERTEX_COUNT,
  HEADER_SDF_INSTANCE_COUNT,
  HEADER_WORLD_WIDTH,
  INSTANCE_FLOATS,
  EFFECTS_VERTEX_FLOATS,
  SDF_INSTANCE_FLOATS,
  ProtocolLayout,
} from '@zap/web';
import type { Renderer } from '@zap/web';

const MANIFEST_URL = '/examples/zap-engine-template/public/assets/assets.json';
const ASSET_BASE = '/examples/zap-engine-template/public/assets/';

async function main() {
  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  if (!canvas) {
    throw new Error('Canvas element not found');
  }

  canvas.width = window.innerWidth * devicePixelRatio;
  canvas.height = window.innerHeight * devicePixelRatio;

  const manifest = await loadManifest(MANIFEST_URL);
  const atlasBlobs = await loadAssetBlobs(manifest, ASSET_BASE);

  const worker = new Worker(
    new URL('../../packages/zap-web/src/worker/engine.worker.ts', import.meta.url),
    { type: 'module' },
  );

  let sharedF32: Float32Array | null = null;
  let renderer: Renderer | null = null;
  let layout: ProtocolLayout | null = null;

  worker.onmessage = async (e: MessageEvent) => {
    const { type } = e.data;

    if (type === 'ready') {
      if (e.data.sharedBuffer) {
        sharedF32 = new Float32Array(e.data.sharedBuffer);
        layout = ProtocolLayout.fromHeader(sharedF32);
      } else {
        layout = new ProtocolLayout(
          e.data.maxInstances,
          e.data.maxEffectsVertices,
          e.data.maxSounds,
          e.data.maxEvents,
          e.data.maxSdfInstances,
        );
      }

      renderer = await initRenderer({
        canvas,
        manifest,
        atlasBlobs,
        gameWidth: 800,
        gameHeight: 600,
        maxInstances: layout.maxInstances,
        maxEffectsVertices: layout.maxEffectsVertices,
        maxSdfInstances: layout.maxSdfInstances,
      });

      requestAnimationFrame(renderLoop);
    } else if (type === 'frame' && !sharedF32) {
      const buf = new Float32Array(e.data.buffer);
      drawFromBuffer(buf);
    }
  };

  function drawFromBuffer(buf: Float32Array) {
    if (!renderer || !layout) return;

    const instanceCount = buf[HEADER_INSTANCE_COUNT];
    const atlasSplit = buf[HEADER_ATLAS_SPLIT];
    const effectsVertexCount = buf[HEADER_EFFECTS_VERTEX_COUNT];
    const sdfInstanceCount = buf[HEADER_SDF_INSTANCE_COUNT];

    if (instanceCount > 0 || sdfInstanceCount > 0) {
      const instanceData = buf.subarray(
        layout.instanceDataOffset,
        layout.instanceDataOffset + instanceCount * INSTANCE_FLOATS,
      );

      let effectsData: Float32Array | undefined;
      if (effectsVertexCount > 0) {
        effectsData = buf.subarray(
          layout.effectsDataOffset,
          layout.effectsDataOffset + effectsVertexCount * EFFECTS_VERTEX_FLOATS,
        );
      }

      let sdfData: Float32Array | undefined;
      if (sdfInstanceCount > 0) {
        sdfData = buf.subarray(
          layout.sdfDataOffset,
          layout.sdfDataOffset + sdfInstanceCount * SDF_INSTANCE_FLOATS,
        );
      }

      renderer.draw(instanceData, instanceCount, atlasSplit, effectsData, effectsVertexCount, sdfData, sdfInstanceCount);
    }
  }

  function renderLoop() {
    if (sharedF32) {
      drawFromBuffer(sharedF32);
    }
    requestAnimationFrame(renderLoop);
  }

  window.addEventListener('resize', () => {
    canvas.width = window.innerWidth * devicePixelRatio;
    canvas.height = window.innerHeight * devicePixelRatio;
    renderer?.resize(canvas.width, canvas.height);
  });

  canvas.addEventListener('pointerdown', (e) => {
    worker.postMessage({ type: 'pointer_down', x: e.offsetX, y: e.offsetY });
  });

  canvas.addEventListener('pointerup', (e) => {
    worker.postMessage({ type: 'pointer_up', x: e.offsetX, y: e.offsetY });
  });

  canvas.addEventListener('pointermove', (e) => {
    worker.postMessage({ type: 'pointer_move', x: e.offsetX, y: e.offsetY });
  });

  worker.postMessage({
    type: 'init',
    wasmUrl: '/examples/zap-engine-template/pkg/zap_engine_template.js',
  });
}

main().catch(console.error);
