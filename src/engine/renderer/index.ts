// Renderer factory — loads assets, tries WebGPU, falls back to Canvas 2D.

export type { Renderer } from './types';

import { initWebGPURenderer } from './webgpu';
import { initCanvas2DRenderer } from './canvas2d';
import type { Renderer } from './types';
import type { AssetManifest } from '../assets/manifest';

/** Probe WebGPU on a disposable canvas to avoid locking the real canvas. */
async function probeWebGPU(): Promise<boolean> {
  if (!navigator.gpu) {
    console.warn('[probeWebGPU] navigator.gpu is undefined');
    return false;
  }
  try {
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) {
      console.warn('[probeWebGPU] requestAdapter returned null');
      return false;
    }
    const device = await adapter.requestDevice();
    const probe = document.createElement('canvas');
    probe.width = probe.height = 1;
    const ctx = probe.getContext('webgpu');
    if (!ctx) {
      device.destroy();
      return false;
    }
    const format = navigator.gpu.getPreferredCanvasFormat();
    ctx.configure({ device, format, alphaMode: 'premultiplied' });
    ctx.unconfigure();
    device.destroy();
    return true;
  } catch (e) {
    console.warn('[probeWebGPU] Failed:', e);
    return false;
  }
}

export interface RendererConfig {
  canvas: HTMLCanvasElement;
  manifest: AssetManifest;
  atlasBlobs: Map<string, Blob>;
  gameWidth: number;
  gameHeight: number;
  force2D?: boolean;
}

/**
 * Initialize the renderer.
 * If force2D is true, skips WebGPU entirely.
 * If WebGPU fails after touching the canvas, throws so the caller can
 * remount the canvas before retrying with force2D=true.
 */
export async function initRenderer(config: RendererConfig): Promise<Renderer> {
  const { canvas, manifest, atlasBlobs, gameWidth, gameHeight, force2D = false } = config;

  if (!force2D) {
    const webgpuAvailable = await probeWebGPU();
    if (webgpuAvailable) {
      try {
        return await initWebGPURenderer({ canvas, manifest, atlasBlobs, gameWidth, gameHeight });
      } catch (e) {
        console.warn('[renderer] WebGPU init failed:', e);
        throw new Error('WebGPUInitFailed');
      }
    }
  }

  console.warn('[renderer] Using Canvas 2D fallback (no HDR/EDR)');
  return initCanvas2DRenderer({ canvas, manifest, atlasBlobs, gameWidth, gameHeight });
}
