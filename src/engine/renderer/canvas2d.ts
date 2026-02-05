// Canvas 2D fallback renderer — used when WebGPU is unavailable.
// Manifest-driven: accepts N atlases as HTMLImageElements.

import { computeProjection } from './camera';
import { SEGMENT_COLORS_RGB8 } from './constants';
import type { Renderer } from './types';
import type { AssetManifest } from '../assets/manifest';
import { createImageFromBlob } from '../assets/loader';

const INSTANCE_FLOATS = 8;
const EFFECTS_VERTEX_FLOATS = 5;
const SDF_INSTANCE_FLOATS = 12;

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

  function drawSdfInstance(
    c: CanvasRenderingContext2D,
    data: Float32Array,
    off: number,
  ) {
    const x = data[off];
    const y = data[off + 1];
    const radius = data[off + 2];
    const r = Math.round(data[off + 4] * 255);
    const g = Math.round(data[off + 5] * 255);
    const b = Math.round(data[off + 6] * 255);

    if (radius <= 0) return;

    // Radial gradient: white highlight center → base color → darkened edge
    const grad = c.createRadialGradient(
      x - radius * 0.3, y - radius * 0.3, radius * 0.1,
      x, y, radius,
    );
    grad.addColorStop(0, `rgba(255, 255, 255, 0.6)`);
    grad.addColorStop(0.4, `rgb(${r}, ${g}, ${b})`);
    const darkR = Math.round(r * 0.3);
    const darkG = Math.round(g * 0.3);
    const darkB = Math.round(b * 0.3);
    grad.addColorStop(1, `rgb(${darkR}, ${darkG}, ${darkB})`);

    c.beginPath();
    c.arc(x, y, radius, 0, Math.PI * 2);
    c.fillStyle = grad;
    c.fill();
  }

  function draw(
    instanceData: Float32Array,
    instanceCount: number,
    atlasSplit: number,
    effectsData?: Float32Array,
    effectsVertexCount?: number,
    sdfData?: Float32Array,
    sdfInstanceCount?: number,
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

    // Atlas 0 instances
    for (let i = 0; i < atlasSplit; i++) {
      drawInstance(ctx!, instanceData, i * INSTANCE_FLOATS, 0);
    }

    // Atlas 1+ instances
    const secondAtlas = manifest.atlases.length > 1 ? 1 : 0;
    for (let i = atlasSplit; i < instanceCount; i++) {
      drawInstance(ctx!, instanceData, i * INSTANCE_FLOATS, secondAtlas);
    }

    // SDF molecules (drawn between sprites and effects)
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
