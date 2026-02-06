// ZapEngine — TypeScript entry point
// Public re-exports for game code to import.

export { initRenderer } from './renderer/index';
export type { Renderer, RendererConfig } from './renderer/index';
export type { Renderer as RendererInterface, LayerBatchDescriptor, BakeState, LightingState } from './renderer/types';

export { loadManifest } from './assets/manifest';
export type { AssetManifest, AtlasDescriptor, SpriteDescriptor } from './assets/manifest';

export { loadAssetBlobs, loadNormalMapBlobs, createGPUTextureFromBlob, createImageFromBlob } from './assets/loader';

export { SoundManager } from './audio/sound-manager';
export type { SoundConfig, SoundEntry } from './audio/sound-manager';
export { buildSoundConfigFromManifest } from './audio/helpers';

export {
  HEADER_FLOATS,
  HEADER_INSTANCE_COUNT,
  HEADER_ATLAS_SPLIT,
  HEADER_EFFECTS_VERTEX_COUNT,
  HEADER_WORLD_WIDTH,
  HEADER_WORLD_HEIGHT,
  HEADER_SOUND_COUNT,
  HEADER_MAX_INSTANCES,
  HEADER_MAX_EFFECTS_VERTICES,
  HEADER_MAX_SOUNDS,
  HEADER_MAX_EVENTS,
  HEADER_EVENT_COUNT,
  HEADER_PROTOCOL_VERSION,
  HEADER_MAX_SDF_INSTANCES,
  HEADER_SDF_INSTANCE_COUNT,
  HEADER_MAX_VECTOR_VERTICES,
  HEADER_VECTOR_VERTEX_COUNT,
  HEADER_MAX_LAYER_BATCHES,
  HEADER_LAYER_BATCH_COUNT,
  HEADER_LAYER_BATCH_OFFSET,
  HEADER_BAKE_STATE,
  HEADER_MAX_LIGHTS,
  HEADER_LIGHT_COUNT,
  HEADER_AMBIENT_R,
  HEADER_AMBIENT_G,
  HEADER_AMBIENT_B,
  PROTOCOL_VERSION,
  INSTANCE_FLOATS,
  EFFECTS_VERTEX_FLOATS,
  EVENT_FLOATS,
  SDF_INSTANCE_FLOATS,
  VECTOR_VERTEX_FLOATS,
  LAYER_BATCH_FLOATS,
  LIGHT_FLOATS,
  DEFAULT_MAX_LAYER_BATCHES,
  DEFAULT_MAX_LIGHTS,
  ProtocolLayout,
} from './worker/protocol';

export { SEGMENT_COLORS, SEGMENT_COLORS_RGB8, packColorsForGPU } from './renderer/constants';
export { computeProjection, buildProjectionMatrix } from './renderer/camera';
