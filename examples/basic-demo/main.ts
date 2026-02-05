// Basic Demo — TypeScript entry point
// Initializes the engine worker, renderer, and connects them.

import {
  initRenderer,
  loadManifest,
  loadAssetBlobs,
  HEADER_FLOATS,
  HEADER_INSTANCE_COUNT,
  HEADER_ATLAS_SPLIT,
  HEADER_EFFECTS_VERTEX_COUNT,
  HEADER_WORLD_WIDTH,
  HEADER_WORLD_HEIGHT,
  INSTANCE_FLOATS,
  EFFECTS_VERTEX_FLOATS,
  INSTANCE_DATA_OFFSET,
  EFFECTS_DATA_OFFSET,
  INSTANCE_DATA_FLOATS,
  MAX_EFFECTS_VERTICES,
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
  let gameWidth = 800;
  let gameHeight = 600;

  worker.onmessage = async (e: MessageEvent) => {
    const { type } = e.data;

    if (type === 'ready') {
      if (e.data.sharedBuffer) {
        sharedF32 = new Float32Array(e.data.sharedBuffer);
        sharedI32 = new Int32Array(e.data.sharedBuffer);
      }

      // Read world dimensions from the first frame (or use defaults)
      gameWidth = sharedF32?.[HEADER_WORLD_WIDTH] || 800;
      gameHeight = sharedF32?.[HEADER_WORLD_HEIGHT] || 600;

      // Init renderer
      renderer = await initRenderer({
        canvas,
        manifest,
        atlasBlobs,
        gameWidth: 800,
        gameHeight: 600,
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
    if (!renderer) return;

    const instanceCount = buf[HEADER_INSTANCE_COUNT];
    const atlasSplit = buf[HEADER_ATLAS_SPLIT];
    const effectsVertexCount = buf[HEADER_EFFECTS_VERTEX_COUNT];

    if (instanceCount > 0) {
      const instanceData = buf.subarray(
        INSTANCE_DATA_OFFSET,
        INSTANCE_DATA_OFFSET + instanceCount * INSTANCE_FLOATS,
      );

      let effectsData: Float32Array | undefined;
      if (effectsVertexCount > 0) {
        effectsData = buf.subarray(
          EFFECTS_DATA_OFFSET,
          EFFECTS_DATA_OFFSET + effectsVertexCount * EFFECTS_VERTEX_FLOATS,
        );
      }

      renderer.draw(instanceData, instanceCount, atlasSplit, effectsData, effectsVertexCount);
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
