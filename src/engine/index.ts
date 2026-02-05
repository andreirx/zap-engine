// ZapEngine — TypeScript entry point
// Public re-exports for game code to import.

export { initRenderer } from './renderer/index';
export type { Renderer, RendererConfig } from './renderer/index';
export type { Renderer as RendererInterface } from './renderer/types';

export { loadManifest } from './assets/manifest';
export type { AssetManifest, AtlasDescriptor, SpriteDescriptor } from './assets/manifest';

export { loadAssetBlobs, createGPUTextureFromBlob, createImageFromBlob } from './assets/loader';

export { SoundManager } from './audio/sound-manager';
export type { SoundConfig } from './audio/sound-manager';

export {
  HEADER_FLOATS,
  HEADER_INSTANCE_COUNT,
  HEADER_ATLAS_SPLIT,
  HEADER_EFFECTS_VERTEX_COUNT,
  HEADER_WORLD_WIDTH,
  HEADER_WORLD_HEIGHT,
  HEADER_SOUND_COUNT,
  INSTANCE_FLOATS,
  EFFECTS_VERTEX_FLOATS,
  INSTANCE_DATA_OFFSET,
  EFFECTS_DATA_OFFSET,
  INSTANCE_DATA_FLOATS,
  MAX_INSTANCES,
  MAX_EFFECTS_VERTICES,
  BUFFER_TOTAL_FLOATS,
} from './worker/protocol';

export { SEGMENT_COLORS, SEGMENT_COLORS_RGB8, packColorsForGPU } from './renderer/constants';
export { computeProjection, buildProjectionMatrix } from './renderer/camera';
