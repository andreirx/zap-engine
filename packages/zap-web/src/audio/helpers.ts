// Audio helper utilities — bridges the AssetManifest sounds section to SoundConfig.

import type { AssetManifest } from '../assets/manifest';
import type { SoundConfig } from './sound-manager';

/**
 * Build a SoundConfig from a manifest's sounds section.
 *
 * Iterates manifest.sounds and creates entries for each sound descriptor
 * that has an event_id. Sounds without event_id are skipped (they can
 * still be loaded manually or used for music).
 *
 * @param manifest   The parsed AssetManifest
 * @param basePath   Base path prefix for audio files (default: '/assets/')
 * @param musicPath  Optional background music path (relative to basePath)
 * @param musicVolume Optional music volume (0.0 - 1.0, default 0.3)
 */
export function buildSoundConfigFromManifest(
  manifest: AssetManifest,
  basePath: string = '/assets/',
  musicPath?: string,
  musicVolume?: number,
): SoundConfig {
  const sounds: Record<number, string> = {};

  if (manifest.sounds) {
    for (const descriptor of Object.values(manifest.sounds)) {
      if (descriptor.event_id != null) {
        sounds[descriptor.event_id] = descriptor.path;
      }
    }
  }

  return {
    sounds,
    basePath,
    musicPath,
    musicVolume,
  };
}
