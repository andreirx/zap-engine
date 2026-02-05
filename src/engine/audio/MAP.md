# audio/

Web Audio API sound management. Configurable — accepts a sound map instead of hardcoded names.

## Files

| File | Purpose |
|---|---|
| `sound-manager.ts` | `SoundManager` class: preload, play by ID/name, background music loop |

## Usage

Games provide a `Record<number, SoundDescriptor>` mapping event IDs to audio file paths.
The engine forwards numeric sound events from Rust; the SoundManager resolves them to audio buffers.
