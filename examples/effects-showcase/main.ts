// Effects Showcase — standalone loader with programmatic atlas.
// No external PNG assets needed: generates a 4x4 colored grid atlas.

import {
  initRenderer,
  ProtocolLayout,
  readFrameState,
} from '@zap/web';
import type { Renderer } from '@zap/web';
import type { AssetManifest } from '../../packages/zap-web/src/assets/manifest';

// Generate a 256x256 atlas (4 cols x 4 rows = 64x64 cells) as a Blob.
// Row 0: terrain tiles (muted colors), Row 1: orb (bright circle), Row 2: UI marker, Row 3: spare.
async function generateAtlas(): Promise<Blob> {
  const size = 256;
  const cellSize = 64;
  const canvas = new OffscreenCanvas(size, size);
  const ctx = canvas.getContext('2d')!;

  // Clear to transparent
  ctx.clearRect(0, 0, size, size);

  // Row 0: terrain tiles — muted colored squares
  const terrainColors = ['#2a3a2a', '#3a2a2a', '#2a2a3a', '#3a3a2a'];
  for (let i = 0; i < 4; i++) {
    ctx.fillStyle = terrainColors[i];
    ctx.fillRect(i * cellSize, 0, cellSize, cellSize);
  }

  // Row 1: glowing orb (radial gradient circle) — used as additive sprite
  for (let i = 0; i < 4; i++) {
    const cx = i * cellSize + cellSize / 2;
    const cy = cellSize + cellSize / 2;
    const r = cellSize / 2 - 4;
    const grad = ctx.createRadialGradient(cx, cy, 0, cx, cy, r);
    grad.addColorStop(0, 'rgba(255, 255, 255, 1.0)');
    grad.addColorStop(0.3, 'rgba(100, 200, 255, 0.8)');
    grad.addColorStop(1, 'rgba(50, 100, 200, 0.0)');
    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.fill();
  }

  // Row 2: UI marker — bright white square with border
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(4, cellSize * 2 + 4, cellSize - 8, cellSize - 8);
  ctx.fillStyle = '#ffcc00';
  ctx.fillRect(cellSize + 4, cellSize * 2 + 4, cellSize - 8, cellSize - 8);

  return canvas.convertToBlob({ type: 'image/png' });
}

const MANIFEST: AssetManifest = {
  atlases: [{
    name: 'main',
    path: 'generated',
    cols: 4,
    rows: 4,
  }],
  sprites: {},
};

async function main() {
  const canvas = document.getElementById('game-canvas') as HTMLCanvasElement;
  if (!canvas) throw new Error('Canvas not found');

  canvas.width = window.innerWidth * devicePixelRatio;
  canvas.height = window.innerHeight * devicePixelRatio;

  // Generate atlas programmatically
  const atlasBlob = await generateAtlas();
  const atlasBlobs = new Map<string, Blob>();
  atlasBlobs.set('main', atlasBlob);

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
          e.data.maxVectorVertices ?? 0,
          e.data.maxLayerBatches,
          e.data.maxLights,
          e.data.maxAlphaEffectsVertices,
          undefined,
          e.data.visibilityCols ?? 0,
          e.data.visibilityRows ?? 0,
        );
      }

      // Tell worker the canvas CSS dimensions for coordinate conversion
      worker.postMessage({ type: 'resize', width: canvas.clientWidth, height: canvas.clientHeight });

      renderer = await initRenderer({
        canvas,
        manifest: MANIFEST,
        atlasBlobs,
        gameWidth: 800,
        gameHeight: 600,
        maxInstances: layout.maxInstances,
        maxEffectsVertices: layout.maxEffectsVertices,
        maxSdfInstances: layout.maxSdfInstances,
        maxVectorVertices: layout.maxVectorVertices,
        maxAlphaEffectsVertices: layout.maxAlphaEffectsVertices,
      });

      requestAnimationFrame(renderLoop);
    } else if (type === 'frame' && !sharedF32) {
      const buf = new Float32Array(e.data.buffer);
      drawFromBuffer(buf);
    }
  };

  function drawFromBuffer(buf: Float32Array) {
    if (!renderer || !layout) return;
    const frame = readFrameState(buf, layout);
    if (!frame) return;

    renderer.draw(
      frame.instanceData,
      frame.instanceCount,
      frame.atlasSplit,
      frame.effectsData,
      frame.effectsVertexCount,
      frame.sdfData,
      frame.sdfInstanceCount,
      frame.vectorData,
      frame.vectorVertexCount,
      frame.layerBatches,
      frame.bakeState,
      frame.lightingState,
      frame.alphaEffectsData,
      frame.alphaEffectsVertexCount,
      frame.alphaEffectsBatches,
      frame.visibilityState,
    );
  }

  function renderLoop() {
    if (sharedF32 && layout) {
      drawFromBuffer(sharedF32);
    }
    // Vsync-driven: notify worker to produce next frame
    worker.postMessage({ type: 'vsync' });
    requestAnimationFrame(renderLoop);
  }

  window.addEventListener('resize', () => {
    canvas.width = window.innerWidth * devicePixelRatio;
    canvas.height = window.innerHeight * devicePixelRatio;
    renderer?.resize(canvas.width, canvas.height);
    worker.postMessage({ type: 'resize', width: canvas.clientWidth, height: canvas.clientHeight });
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
    wasmUrl: '/examples/effects-showcase/pkg/effects_showcase.js',
    manifestJson: JSON.stringify(MANIFEST),
  });
}

main().catch(console.error);
