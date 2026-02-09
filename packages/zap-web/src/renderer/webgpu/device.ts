// WebGPU device initialization, context configuration, and tier detection.

import type { RenderTier } from '../types';

export interface GPUDeviceContext {
  device: GPUDevice;
  context: GPUCanvasContext;
  format: GPUTextureFormat;
  tier: RenderTier;
}

/** Per-tier glow multipliers for shader override constants. */
export const GLOW_MULT: Record<Exclude<RenderTier, 'canvas2d'>, { effects: number; sdf: number; vector: number }> = {
  'hdr-edr':  { effects: 6.4, sdf: 5.4, vector: 6.4 },
  'hdr-srgb': { effects: 3.0, sdf: 2.5, vector: 3.0 },
  'sdr':      { effects: 1.0, sdf: 0.5, vector: 1.0 },
};

/**
 * Initialize WebGPU device with progressive HDR/EDR configuration.
 * Tries full HDR/EDR first, then basic rgba16float, then preferred format.
 */
export async function initDevice(canvas: HTMLCanvasElement): Promise<GPUDeviceContext> {
  if (!navigator.gpu) {
    throw new Error('WebGPU not supported');
  }

  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) {
    throw new Error('No WebGPU adapter found');
  }

  const device = await adapter.requestDevice();
  const context = canvas.getContext('webgpu');
  if (!context) {
    throw new Error('Failed to get WebGPU context');
  }

  // Progressive configure — try full HDR/EDR, then basic rgba16float, then preferred format.
  let format: GPUTextureFormat = 'rgba16float';
  let tier: RenderTier = 'sdr';

  try {
    context.configure({
      device,
      format: 'rgba16float',
      colorSpace: 'display-p3',
      toneMapping: { mode: 'extended' },
      alphaMode: 'premultiplied',
    });
    tier = 'hdr-edr';
  } catch {
    try {
      context.configure({
        device,
        format: 'rgba16float',
        alphaMode: 'premultiplied',
      });
      tier = 'hdr-srgb';
    } catch {
      format = navigator.gpu.getPreferredCanvasFormat();
      context.configure({
        device,
        format,
        alphaMode: 'premultiplied',
      });
      tier = 'sdr';
    }
  }

  console.info(`[renderer] WebGPU tier: ${tier} (format: ${format})`);

  return { device, context, format, tier };
}

/**
 * Resize canvas and reconfigure context with tier-appropriate settings.
 */
export function resizeContext(
  canvas: HTMLCanvasElement,
  context: GPUCanvasContext,
  device: GPUDevice,
  format: GPUTextureFormat,
  tier: RenderTier,
  width: number,
  height: number,
): void {
  canvas.width = width;
  canvas.height = height;
  const configOpts: GPUCanvasConfiguration = { device, format, alphaMode: 'premultiplied' };
  if (tier === 'hdr-edr') {
    configOpts.colorSpace = 'display-p3';
    configOpts.toneMapping = { mode: 'extended' };
  }
  context.configure(configOpts);
}
