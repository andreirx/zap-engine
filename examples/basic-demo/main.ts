// Basic Demo — TypeScript entry point
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
} from '../../src/engine/index';
import type { Renderer } from '../../src/engine/index';

const MANIFEST_URL = '/examples/basic-demo/public/assets/assets.json';
const ASSET_BASE = '/examples/basic-demo/public/assets/';

async function main() {
  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  if (!canvas) {
    throw new Error('Canvas element not found');
  }

  // Set canvas size
  canvas.width = window.innerWidth * devicePixelRatio;
  canvas.height = window.innerHeight * devicePixelRatio;

  // Load manifest and assets
  const manifest = await loadManifest(MANIFEST_URL);
  const atlasBlobs = await loadAssetBlobs(manifest, ASSET_BASE);

  // Create worker
  const worker = new Worker(
    new URL('../../src/engine/worker/engine.worker.ts', import.meta.url),
    { type: 'module' },
  );

  // Wait for worker ready
  let sharedF32: Float32Array | null = null;
  let sharedI32: Int32Array | null = null;
  let renderer: Renderer | null = null;
  let layout: ProtocolLayout | null = null;
  let gameWidth = 800;
  let gameHeight = 600;

  worker.onmessage = async (e: MessageEvent) => {
    const { type } = e.data;

    if (type === 'ready') {
      if (e.data.sharedBuffer) {
        // SharedArrayBuffer path: read layout from header
        sharedF32 = new Float32Array(e.data.sharedBuffer);
        sharedI32 = new Int32Array(e.data.sharedBuffer);
        layout = ProtocolLayout.fromHeader(sharedF32);
      } else {
        // postMessage fallback: read layout from message data
        layout = new ProtocolLayout(
          e.data.maxInstances,
          e.data.maxEffectsVertices,
          e.data.maxSounds,
          e.data.maxEvents,
          e.data.maxSdfInstances,
        );
      }

      // Read world dimensions from the first frame (or use defaults)
      gameWidth = sharedF32?.[HEADER_WORLD_WIDTH] || 800;
      gameHeight = sharedF32?.[HEADER_WORLD_WIDTH + 1] || 600;

      // Init renderer with dynamic capacities
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

      // Start render loop
      requestAnimationFrame(renderLoop);
    } else if (type === 'frame' && !sharedF32) {
      // postMessage fallback: use the buffer from the message
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

  // Handle resize
  window.addEventListener('resize', () => {
    canvas.width = window.innerWidth * devicePixelRatio;
    canvas.height = window.innerHeight * devicePixelRatio;
    renderer?.resize(canvas.width, canvas.height);
  });

  // Forward pointer events to worker
  canvas.addEventListener('pointerdown', (e) => {
    worker.postMessage({ type: 'pointer_down', x: e.offsetX, y: e.offsetY });
  });

  canvas.addEventListener('pointerup', (e) => {
    worker.postMessage({ type: 'pointer_up', x: e.offsetX, y: e.offsetY });
  });

  canvas.addEventListener('pointermove', (e) => {
    worker.postMessage({ type: 'pointer_move', x: e.offsetX, y: e.offsetY });
  });

  // Initialize worker with WASM URL
  worker.postMessage({
    type: 'init',
    wasmUrl: '/examples/basic-demo/pkg/basic_demo.js',
  });
}

main().catch(console.error);
