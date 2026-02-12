// Canvas 2D fallback renderer — used when WebGPU is unavailable.
// Manifest-driven: accepts N atlases as HTMLImageElements.

import { computeProjection } from './camera';
import { SEGMENT_COLORS_RGB8 } from './constants';
import type { Renderer, LayerBatchDescriptor, BakeState, LightingState } from './types';
import type { AssetManifest } from '../assets/manifest';
import { createImageFromBlob } from '../assets/loader';
import {
  INSTANCE_FLOATS,
  EFFECTS_VERTEX_FLOATS,
  SDF_INSTANCE_FLOATS,
  VECTOR_VERTEX_FLOATS,
} from '../worker/protocol';

export interface Canvas2DRendererConfig {
  canvas: HTMLCanvasElement;
  manifest: AssetManifest;
  atlasBlobs: Map<string, Blob>;
  gameWidth: number;
  gameHeight: number;
}

export async function initCanvas2DRenderer(config: Canvas2DRendererConfig): Promise<Renderer> {
  const { canvas, manifest, atlasBlobs, gameWidth, gameHeight } = config;

  const ctx = canvas.getContext('2d');
  if (!ctx) {
    throw new Error('Failed to get Canvas 2D context');
  }

  // Create HTMLImageElements from blobs
  const images: HTMLImageElement[] = [];
  for (const atlas of manifest.atlases) {
    const blob = atlasBlobs.get(atlas.name);
    if (!blob) {
      throw new Error(`Missing blob for atlas: ${atlas.name}`);
    }
    images.push(await createImageFromBlob(blob));
  }

  // Precompute cell sizes per atlas
  const cellSizes = manifest.atlases.map((atlas, i) => ({
    cellW: images[i].width / atlas.cols,
    cellH: images[i].height / atlas.rows,
    cols: atlas.cols,
  }));

  function drawInstance(
    c: CanvasRenderingContext2D,
    data: Float32Array,
    off: number,
    atlasIdx: number,
  ) {
    const x = data[off];
    const y = data[off + 1];
    const rotation = data[off + 2];
    const scale = data[off + 3];
    const spriteCol = data[off + 4];
    const alpha = data[off + 5];
    const cellSpan = data[off + 6];
    const atlasRow = data[off + 7];

    if (alpha <= 0) return;
    if (atlasIdx >= images.length) return;

    const img = images[atlasIdx];
    const { cellW, cellH, cols } = cellSizes[atlasIdx];

    const cellCount = Math.max(cellSpan, 1);
    const col = spriteCol % cols;

    const srcX = col * cellW;
    const srcY = atlasRow * cellH;
    const srcW = cellCount * cellW;
    const srcH = cellCount * cellH;

    const size = scale;
    const half = size / 2;

    c.globalAlpha = Math.min(alpha, 1);

    if (rotation === 0) {
      c.drawImage(img, srcX, srcY, srcW, srcH, x - half, y - half, size, size);
    } else {
      c.save();
      c.translate(x, y);
      c.rotate(rotation);
      c.drawImage(img, srcX, srcY, srcW, srcH, -half, -half, size, size);
      c.restore();
    }
  }

  function drawEffectsTriangle(
    c: CanvasRenderingContext2D,
    data: Float32Array,
    v0Index: number,
  ) {
    const off0 = v0Index * EFFECTS_VERTEX_FLOATS;
    const off1 = (v0Index + 1) * EFFECTS_VERTEX_FLOATS;
    const off2 = (v0Index + 2) * EFFECTS_VERTEX_FLOATS;

    const x0 = data[off0], y0 = data[off0 + 1];
    const x1 = data[off1], y1 = data[off1 + 1];
    const x2 = data[off2], y2 = data[off2 + 1];

    const colorIdx = Math.round(data[off0 + 2]);
    const [r, g, b] = SEGMENT_COLORS_RGB8[Math.min(Math.max(colorIdx, 0), SEGMENT_COLORS_RGB8.length - 1)];

    const u0 = data[off0 + 3], u1 = data[off1 + 3], u2 = data[off2 + 3];
    const v0v = data[off0 + 4], v1v = data[off1 + 4], v2v = data[off2 + 4];
    const avgU = (u0 + u1 + u2) / 3;
    const avgV = (v0v + v1v + v2v) / 3;

    const d = Math.abs(avgU * 2 - 1);
    const halo = Math.exp(-d * d * 3);
    const a = halo * avgV;
    if (a < 0.02) return;

    c.beginPath();
    c.moveTo(x0, y0);
    c.lineTo(x1, y1);
    c.lineTo(x2, y2);
    c.closePath();
    c.fillStyle = `rgba(${r}, ${g}, ${b}, ${Math.min(a, 1).toFixed(3)})`;
    c.fill();
  }

  function drawRoundedRect(
    c: CanvasRenderingContext2D,
    cx: number,
    cy: number,
    halfW: number,
    halfH: number,
    cornerR: number,
    rotation: number,
  ) {
    const cr = Math.min(cornerR, halfW, halfH);
    c.save();
    c.translate(cx, cy);
    if (rotation !== 0) c.rotate(rotation);
    c.beginPath();
    c.moveTo(-halfW + cr, -halfH);
    c.arcTo(halfW, -halfH, halfW, halfH, cr);
    c.arcTo(halfW, halfH, -halfW, halfH, cr);
    c.arcTo(-halfW, halfH, -halfW, -halfH, cr);
    c.arcTo(-halfW, -halfH, halfW, -halfH, cr);
    c.closePath();
    c.restore();
  }

  function drawSdfInstance(
    c: CanvasRenderingContext2D,
    data: Float32Array,
    off: number,
  ) {
    const x = data[off];
    const y = data[off + 1];
    const radius = data[off + 2];
    const rotation = data[off + 3];
    const r = Math.round(data[off + 4] * 255);
    const g = Math.round(data[off + 5] * 255);
    const b = Math.round(data[off + 6] * 255);
    const shapeType = data[off + 9];
    const halfHeight = data[off + 10];
    const cornerRadius = data[off + 11];

    if (radius <= 0) return;

    const darkR = Math.round(r * 0.3);
    const darkG = Math.round(g * 0.3);
    const darkB = Math.round(b * 0.3);

    if (shapeType < 0.5) {
      // ---- Sphere ----
      const extra = data[off + 11];  // Stripe flag for pool balls
      const isStriped = extra > 0.5;

      if (isStriped) {
        // Striped pool ball: white with colored horizontal band in middle
        c.save();
        c.translate(x, y);
        if (rotation !== 0) c.rotate(rotation);

        // Base white sphere with shading
        const whiteGrad = c.createRadialGradient(
          -radius * 0.3, -radius * 0.3, radius * 0.1,
          0, 0, radius,
        );
        whiteGrad.addColorStop(0, 'rgba(255, 255, 255, 0.9)');
        whiteGrad.addColorStop(0.4, 'rgb(245, 245, 245)');
        whiteGrad.addColorStop(1, 'rgb(180, 180, 180)');
        c.beginPath();
        c.arc(0, 0, radius, 0, Math.PI * 2);
        c.fillStyle = whiteGrad;
        c.fill();

        // Colored stripe band in the middle (|y| < 0.35 * radius)
        const stripeWidth = radius * 0.7;  // Total stripe height
        c.beginPath();
        c.rect(-radius, -stripeWidth / 2, radius * 2, stripeWidth);
        c.clip();

        const stripeGrad = c.createRadialGradient(
          -radius * 0.3, -radius * 0.3, radius * 0.1,
          0, 0, radius,
        );
        stripeGrad.addColorStop(0, `rgba(255, 255, 255, 0.5)`);
        stripeGrad.addColorStop(0.4, `rgb(${r}, ${g}, ${b})`);
        stripeGrad.addColorStop(1, `rgb(${darkR}, ${darkG}, ${darkB})`);
        c.beginPath();
        c.arc(0, 0, radius, 0, Math.PI * 2);
        c.fillStyle = stripeGrad;
        c.fill();

        c.restore();
      } else {
        // Solid sphere: radial gradient circle
        const grad = c.createRadialGradient(
          x - radius * 0.3, y - radius * 0.3, radius * 0.1,
          x, y, radius,
        );
        grad.addColorStop(0, `rgba(255, 255, 255, 0.6)`);
        grad.addColorStop(0.4, `rgb(${r}, ${g}, ${b})`);
        grad.addColorStop(1, `rgb(${darkR}, ${darkG}, ${darkB})`);

        c.beginPath();
        c.arc(x, y, radius, 0, Math.PI * 2);
        c.fillStyle = grad;
        c.fill();
      }
    } else if (shapeType < 1.5) {
      // ---- Capsule: rounded rect with linear gradient ----
      const halfW = radius;
      const halfH = radius + halfHeight;

      // Linear gradient perpendicular to the capsule axis (left-to-right in local space)
      c.save();
      c.translate(x, y);
      if (rotation !== 0) c.rotate(rotation);

      const grad = c.createLinearGradient(-halfW, 0, halfW, 0);
      grad.addColorStop(0, `rgb(${darkR}, ${darkG}, ${darkB})`);
      grad.addColorStop(0.3, `rgb(${r}, ${g}, ${b})`);
      grad.addColorStop(0.5, `rgba(255, 255, 255, 0.4)`);
      grad.addColorStop(0.7, `rgb(${r}, ${g}, ${b})`);
      grad.addColorStop(1, `rgb(${darkR}, ${darkG}, ${darkB})`);

      // Draw rounded rect at origin (already translated)
      const cr = Math.min(radius, halfW, halfH);
      c.beginPath();
      c.moveTo(-halfW + cr, -halfH);
      c.arcTo(halfW, -halfH, halfW, halfH, cr);
      c.arcTo(halfW, halfH, -halfW, halfH, cr);
      c.arcTo(-halfW, halfH, -halfW, -halfH, cr);
      c.arcTo(-halfW, -halfH, halfW, -halfH, cr);
      c.closePath();
      c.fillStyle = grad;
      c.fill();
      c.restore();
    } else {
      // ---- RoundedBox: rounded rect with gradient ----
      const halfW = radius;
      const halfH = radius + halfHeight;
      const cr = cornerRadius;

      c.save();
      c.translate(x, y);
      if (rotation !== 0) c.rotate(rotation);

      const grad = c.createLinearGradient(-halfW, -halfH, halfW, halfH);
      grad.addColorStop(0, `rgba(255, 255, 255, 0.4)`);
      grad.addColorStop(0.3, `rgb(${r}, ${g}, ${b})`);
      grad.addColorStop(1, `rgb(${darkR}, ${darkG}, ${darkB})`);

      const clamped = Math.min(cr, halfW, halfH);
      c.beginPath();
      c.moveTo(-halfW + clamped, -halfH);
      c.arcTo(halfW, -halfH, halfW, halfH, clamped);
      c.arcTo(halfW, halfH, -halfW, halfH, clamped);
      c.arcTo(-halfW, halfH, -halfW, -halfH, clamped);
      c.arcTo(-halfW, -halfH, halfW, -halfH, clamped);
      c.closePath();
      c.fillStyle = grad;
      c.fill();
      c.restore();
    }
  }

  function drawVectorTriangle(
    c: CanvasRenderingContext2D,
    data: Float32Array,
    v0Index: number,
  ) {
    const off0 = v0Index * VECTOR_VERTEX_FLOATS;
    const off1 = (v0Index + 1) * VECTOR_VERTEX_FLOATS;
    const off2 = (v0Index + 2) * VECTOR_VERTEX_FLOATS;

    const x0 = data[off0], y0 = data[off0 + 1];
    const x1 = data[off1], y1 = data[off1 + 1];
    const x2 = data[off2], y2 = data[off2 + 1];

    // Average the vertex colors (simple flat shading)
    const r = Math.round(((data[off0 + 2] + data[off1 + 2] + data[off2 + 2]) / 3) * 255);
    const g = Math.round(((data[off0 + 3] + data[off1 + 3] + data[off2 + 3]) / 3) * 255);
    const b = Math.round(((data[off0 + 4] + data[off1 + 4] + data[off2 + 4]) / 3) * 255);
    const a = (data[off0 + 5] + data[off1 + 5] + data[off2 + 5]) / 3;

    if (a < 0.01) return;

    c.beginPath();
    c.moveTo(x0, y0);
    c.lineTo(x1, y1);
    c.lineTo(x2, y2);
    c.closePath();
    c.fillStyle = `rgba(${r}, ${g}, ${b}, ${Math.min(a, 1).toFixed(3)})`;
    c.fill();
  }

  function drawBatchRange(
    c: CanvasRenderingContext2D,
    instanceData: Float32Array,
    batchStart: number,
    batchEnd: number,
    batchAtlasSplit: number,
  ) {
    const secondAtlas = manifest.atlases.length > 1 ? 1 : 0;
    // Atlas 0 portion
    for (let i = batchStart; i < batchAtlasSplit; i++) {
      drawInstance(c, instanceData, i * INSTANCE_FLOATS, 0);
    }
    // Atlas 1+ portion
    for (let i = batchAtlasSplit; i < batchEnd; i++) {
      drawInstance(c, instanceData, i * INSTANCE_FLOATS, secondAtlas);
    }
  }

  // ---- Layer baking cache (OffscreenCanvas per baked layer) ----
  interface Canvas2DLayerCache {
    offscreen: OffscreenCanvas;
    offCtx: OffscreenCanvasRenderingContext2D;
    lastBakeGen: number;
    width: number;
    height: number;
  }
  const layerCaches = new Map<number, Canvas2DLayerCache>();

  function getOrCreateLayerCache(layerId: number, w: number, h: number, scaleX: number, scaleY: number): Canvas2DLayerCache {
    let cache = layerCaches.get(layerId);
    if (cache && cache.width === w && cache.height === h) {
      return cache;
    }
    // Create or recreate at new size
    const offscreen = new OffscreenCanvas(w, h);
    const offCtx = offscreen.getContext('2d')!;
    cache = { offscreen, offCtx, lastBakeGen: -1, width: w, height: h };
    layerCaches.set(layerId, cache);
    return cache;
  }

  function draw(
    instanceData: Float32Array,
    instanceCount: number,
    atlasSplit: number,
    effectsData?: Float32Array,
    effectsVertexCount?: number,
    sdfData?: Float32Array,
    sdfInstanceCount?: number,
    vectorData?: Float32Array,
    vectorVertexCount?: number,
    layerBatches?: LayerBatchDescriptor[],
    bakeState?: BakeState,
    _lightingState?: LightingState,
  ) {
    const w = canvas.width;
    const h = canvas.height;
    const { scaleX, scaleY } = computeProjection(w, h, gameWidth, gameHeight);

    ctx!.globalCompositeOperation = 'source-over';
    ctx!.globalAlpha = 1;
    ctx!.fillStyle = '#05050d';
    ctx!.fillRect(0, 0, w, h);

    ctx!.save();
    ctx!.scale(scaleX, scaleY);

    const hasBaking = bakeState && bakeState.bakedMask !== 0 && layerBatches && layerBatches.length > 0;

    // Draw sprite instances — use layer batches if available, else legacy path
    if (layerBatches && layerBatches.length > 0) {
      for (const batch of layerBatches) {
        if (hasBaking && (bakeState!.bakedMask & (1 << batch.layerId)) !== 0) {
          // Baked layer: render to cache if dirty, then blit
          const cache = getOrCreateLayerCache(batch.layerId, w, h, scaleX, scaleY);
          if (cache.lastBakeGen !== bakeState!.bakeGen) {
            // Re-render to offscreen canvas
            cache.offCtx.clearRect(0, 0, w, h);
            cache.offCtx.save();
            cache.offCtx.scale(scaleX, scaleY);
            drawBatchRange(cache.offCtx as unknown as CanvasRenderingContext2D, instanceData, batch.start, batch.end, batch.atlasSplit);
            cache.offCtx.restore();
            cache.lastBakeGen = bakeState!.bakeGen;
          }
          // Blit cached layer (undo the current scale transform temporarily)
          ctx!.save();
          ctx!.setTransform(1, 0, 0, 1, 0, 0);
          ctx!.drawImage(cache.offscreen, 0, 0);
          ctx!.restore();
        } else {
          // Live layer: draw directly
          drawBatchRange(ctx!, instanceData, batch.start, batch.end, batch.atlasSplit);
        }
      }
    } else {
      drawBatchRange(ctx!, instanceData, 0, instanceCount, atlasSplit);
    }

    // Vector geometry (drawn between sprites and SDF)
    const hasVectors = vectorData && vectorVertexCount && vectorVertexCount > 0;
    if (hasVectors) {
      ctx!.globalAlpha = 1;
      const triCount = Math.floor(vectorVertexCount / 3);
      for (let t = 0; t < triCount; t++) {
        drawVectorTriangle(ctx!, vectorData, t * 3);
      }
    }

    // SDF molecules (drawn between vectors and effects)
    const hasSdf = sdfData && sdfInstanceCount && sdfInstanceCount > 0;
    if (hasSdf) {
      ctx!.globalAlpha = 1;
      for (let i = 0; i < sdfInstanceCount; i++) {
        drawSdfInstance(ctx!, sdfData, i * SDF_INSTANCE_FLOATS);
      }
    }

    // Effects
    const hasEffects = effectsData && effectsVertexCount && effectsVertexCount > 0;
    if (hasEffects) {
      ctx!.globalCompositeOperation = 'lighter';
      ctx!.globalAlpha = 1;
      const triCount = Math.floor(effectsVertexCount / 3);
      for (let t = 0; t < triCount; t++) {
        drawEffectsTriangle(ctx!, effectsData, t * 3);
      }
      ctx!.globalCompositeOperation = 'source-over';
    }

    ctx!.restore();
  }

  function resize(width: number, height: number) {
    canvas.width = width;
    canvas.height = height;
  }

  return { backend: 'canvas2d', tier: 'canvas2d', draw, resize };
}
