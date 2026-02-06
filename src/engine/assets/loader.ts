// Asset loader — fetches atlas PNGs as Blobs and creates renderer-specific resources.

import type { AssetManifest, GPUTextureAsset } from './manifest';

/** Fetch all atlas PNGs as a name→Blob map. */
export async function loadAssetBlobs(
  manifest: AssetManifest,
  basePath: string = '/assets/',
): Promise<Map<string, Blob>> {
  const entries = await Promise.all(
    manifest.atlases.map(async (atlas) => {
      const url = `${basePath}${atlas.path}`;
      const resp = await fetch(url);
      if (!resp.ok) {
        throw new Error(`Failed to fetch atlas ${atlas.name}: HTTP ${resp.status} from ${url}`);
      }
      const blob = await resp.blob();
      return [atlas.name, blob] as const;
    })
  );
  return new Map(entries);
}

/** Fetch normal map PNGs (for atlases that have normalMap defined) as a name→Blob map. */
export async function loadNormalMapBlobs(
  manifest: AssetManifest,
  basePath: string = '/assets/',
): Promise<Map<string, Blob>> {
  const atlasesWithNormals = manifest.atlases.filter((a) => a.normalMap);
  if (atlasesWithNormals.length === 0) return new Map();

  const entries = await Promise.all(
    atlasesWithNormals.map(async (atlas) => {
      const url = `${basePath}${atlas.normalMap}`;
      const resp = await fetch(url);
      if (!resp.ok) {
        throw new Error(`Failed to fetch normal map ${atlas.name}: HTTP ${resp.status} from ${url}`);
      }
      const blob = await resp.blob();
      return [atlas.name, blob] as const;
    })
  );
  return new Map(entries);
}

// ---- WebGPU: Blob → ImageBitmap → GPUTexture ----

export async function createGPUTextureFromBlob(
  device: GPUDevice,
  blob: Blob,
  premultiply: boolean = true,
): Promise<GPUTextureAsset> {
  const bitmap = await createImageBitmap(blob, {
    colorSpaceConversion: 'none',
    premultiplyAlpha: premultiply ? 'premultiply' : 'none',
  });

  const { width, height } = bitmap;

  const texture = device.createTexture({
    size: { width, height },
    format: 'rgba8unorm',
    usage:
      GPUTextureUsage.TEXTURE_BINDING |
      GPUTextureUsage.COPY_DST |
      GPUTextureUsage.RENDER_ATTACHMENT,
  });

  device.queue.copyExternalImageToTexture(
    { source: bitmap },
    { texture },
    { width, height },
  );

  bitmap.close();

  return { texture, view: texture.createView(), width, height };
}

// ---- Canvas 2D: Blob → Object URL → HTMLImageElement ----

export function createImageFromBlob(blob: Blob): Promise<HTMLImageElement> {
  const url = URL.createObjectURL(blob);
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(url);
      resolve(img);
    };
    img.onerror = reject;
    img.src = url;
  });
}
