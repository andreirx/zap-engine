// Asset loader — fetches atlas PNGs as Blobs and creates renderer-specific resources.

import type { AssetManifest, GPUTextureAsset } from './manifest';

/**
 * Fetch atlas PNGs as a name→Blob map.
 *
 * When `preloadedBlobs` is provided, any atlas whose name already exists
 * in that map is used as-is — no network fetch is issued.  This supports
 * a layered asset model where seed atlases come from disk/S3 and overlay
 * atlases (e.g. user-baked characters from IndexedDB) are supplied as
 * in-memory blobs.
 *
 * The returned map contains one entry per manifest atlas: either the
 * pre-loaded blob or the freshly fetched one.
 */
export async function loadAssetBlobs(
  manifest: AssetManifest,
  basePath: string = '/assets/',
  preloadedBlobs?: Map<string, Blob>,
): Promise<Map<string, Blob>> {
  const result = new Map<string, Blob>(preloadedBlobs);

  // Only fetch atlases not already satisfied by preloadedBlobs.
  const toFetch = manifest.atlases.filter((a) => !result.has(a.name));
  if (toFetch.length > 0) {
    const entries = await Promise.all(
      toFetch.map(async (atlas) => {
        const url = `${basePath}${atlas.path}`;
        const resp = await fetch(url);
        if (!resp.ok) {
          throw new Error(`Failed to fetch atlas ${atlas.name}: HTTP ${resp.status} from ${url}`);
        }
        const blob = await resp.blob();
        return [atlas.name, blob] as const;
      })
    );
    for (const [name, blob] of entries) {
      result.set(name, blob);
    }
  }

  return result;
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
