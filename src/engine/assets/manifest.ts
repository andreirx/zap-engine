// Asset manifest types — mirrors Rust assets/manifest.rs.

export interface AtlasDescriptor {
  name: string;
  cols: number;
  rows: number;
  path: string;
  /** Optional path to a normal map PNG (e.g., "tiles_normals.png"). */
  normalMap?: string;
}

export interface SpriteDescriptor {
  atlas: number;
  col: number;
  row: number;
  span?: number;
}

export interface SoundDescriptor {
  path: string;
  event_id?: number;
}

export interface AssetManifest {
  atlases: AtlasDescriptor[];
  sprites: Record<string, SpriteDescriptor>;
  sounds?: Record<string, SoundDescriptor>;
}

/** GPU texture asset with view for WebGPU binding. */
export interface GPUTextureAsset {
  texture: GPUTexture;
  view: GPUTextureView;
  width: number;
  height: number;
}

/** Load and parse an asset manifest from a URL. */
export async function loadManifest(url: string): Promise<AssetManifest> {
  const resp = await fetch(url);
  if (!resp.ok) {
    throw new Error(`Failed to load manifest: HTTP ${resp.status} from ${url}`);
  }
  return resp.json() as Promise<AssetManifest>;
}
