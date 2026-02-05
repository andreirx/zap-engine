# Audio Module

Provides Web Audio API integration for sound effects and background music.

## Files

| File | Purpose |
|------|---------|
| `sound-manager.ts` | `SoundManager` class — decodes audio buffers, plays by event ID with per-sound volume via GainNode, handles background music (looped), AudioContext suspend/resume for browser autoplay policies. |
| `helpers.ts` | `buildSoundConfigFromManifest()` — bridges `AssetManifest.sounds` to `SoundConfig` for zero-config audio setup. |

## Architecture

```
AssetManifest (sounds section)
        │
        ▼
buildSoundConfigFromManifest()  ─── optional convenience
        │
        ▼
    SoundConfig  ─── { sounds: Record<eventId, path|SoundEntry>, musicPath?, basePath? }
        │
        ▼
    SoundManager  ─── Web Audio API (AudioContext + AudioBuffer cache)
        │
    .init()   → fetch + decodeAudioData for all entries
    .play(id) → BufferSourceNode + optional GainNode
    .resume() → unsuspend AudioContext (call on user interaction)
```

## Sound Event Flow

1. Rust game calls `ctx.emit_sound(event_id)` during `Game::update()`
2. `GameRunner` packs sound events into the SharedArrayBuffer sound section
3. Worker reads the section and posts `{ type: 'sound', events: [id, ...] }` to main thread
4. `useZapEngine` hook receives the message and calls `SoundManager.play(id)` for each

## Integration

- **React hook**: `useZapEngine` accepts optional `sounds: SoundConfig` — creates SoundManager, calls `init()` eagerly (AudioContext starts suspended), resumes on first `pointerdown`.
- **Standalone**: Create `SoundManager` directly, call `init()`, wire `play()` to your event handler.
- **Manifest-driven**: Use `buildSoundConfigFromManifest(manifest, basePath)` to auto-map sounds with `event_id` fields.

## SoundEntry Format

Sounds can be specified as a plain path string or as a `SoundEntry` object with volume control:

```typescript
sounds: {
  1: 'click.mp3',                           // simple path, volume = 1.0
  2: { path: 'explosion.mp3', volume: 0.5 }, // with volume
}
```
