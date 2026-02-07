// Sound manager using Web Audio API.
// Configurable — accepts a sound map instead of hardcoded names.

/** Descriptor for a single sound entry with optional volume control. */
export interface SoundEntry {
  /** Audio file path (relative to basePath). */
  path: string;
  /** Playback volume (0.0 - 1.0, default 1.0). */
  volume?: number;
}

export interface SoundConfig {
  /** Map of event ID → audio file path or SoundEntry with volume. */
  sounds: Record<number, string | SoundEntry>;
  /** Optional background music path. */
  musicPath?: string;
  /** Music volume (0.0 - 1.0, default 0.3). */
  musicVolume?: number;
  /** Base path prefix for all audio files (default: '/audio/'). */
  basePath?: string;
}

export class SoundManager {
  private ctx: AudioContext | null = null;
  private buffers: Map<string, AudioBuffer> = new Map();
  private musicSource: AudioBufferSourceNode | null = null;
  private musicGain: GainNode | null = null;
  private loaded = false;
  private config: SoundConfig;

  constructor(config: SoundConfig) {
    this.config = config;
  }

  /** Resolve a sound entry to a normalized { path, volume } object. */
  private resolveSound(eventId: number): { path: string; volume: number } | null {
    const entry = this.config.sounds[eventId];
    if (!entry) return null;
    if (typeof entry === 'string') {
      return { path: entry, volume: 1.0 };
    }
    return { path: entry.path, volume: entry.volume ?? 1.0 };
  }

  async init(): Promise<void> {
    this.ctx = new AudioContext();
    const basePath = this.config.basePath ?? '/audio/';

    const loads: Promise<void>[] = [];

    // Load all sound effects
    for (const key of Object.keys(this.config.sounds)) {
      const resolved = this.resolveSound(Number(key));
      if (resolved) {
        loads.push(this.loadSound(basePath + resolved.path));
      }
    }

    // Load background music if specified
    if (this.config.musicPath) {
      loads.push(this.loadSound(basePath + this.config.musicPath));
    }

    await Promise.all(loads);
    this.loaded = true;
  }

  private async loadSound(fullPath: string): Promise<void> {
    if (!this.ctx) return;
    try {
      const response = await fetch(fullPath);
      const arrayBuffer = await response.arrayBuffer();
      const audioBuffer = await this.ctx.decodeAudioData(arrayBuffer);
      this.buffers.set(fullPath, audioBuffer);
    } catch {
      console.warn(`[SoundManager] Failed to load sound: ${fullPath}`);
    }
  }

  async resume(): Promise<void> {
    if (this.ctx?.state === 'suspended') {
      await this.ctx.resume();
    }
  }

  /** Play a sound event by its numeric ID. */
  play(eventId: number): void {
    const resolved = this.resolveSound(eventId);
    if (!resolved) return;
    const basePath = this.config.basePath ?? '/audio/';
    this.playBuffer(basePath + resolved.path, resolved.volume);
  }

  /** Play a decoded audio buffer with optional volume. */
  private playBuffer(fullPath: string, volume: number): void {
    if (!this.ctx || !this.loaded) return;
    const buffer = this.buffers.get(fullPath);
    if (!buffer) return;

    const source = this.ctx.createBufferSource();
    source.buffer = buffer;

    if (volume < 1.0) {
      const gain = this.ctx.createGain();
      gain.gain.value = volume;
      gain.connect(this.ctx.destination);
      source.connect(gain);
    } else {
      source.connect(this.ctx.destination);
    }
    source.start();
  }

  /** Start background music (looped). */
  startMusic(): void {
    if (!this.ctx || !this.loaded || !this.config.musicPath) return;
    this.stopMusic();

    const basePath = this.config.basePath ?? '/audio/';
    const buffer = this.buffers.get(basePath + this.config.musicPath);
    if (!buffer) return;

    this.musicGain = this.ctx.createGain();
    this.musicGain.gain.value = this.config.musicVolume ?? 0.3;
    this.musicGain.connect(this.ctx.destination);

    this.musicSource = this.ctx.createBufferSource();
    this.musicSource.buffer = buffer;
    this.musicSource.loop = true;
    this.musicSource.connect(this.musicGain);
    this.musicSource.start();
  }

  /** Stop background music. */
  stopMusic(): void {
    if (this.musicSource) {
      try {
        this.musicSource.stop();
      } catch {
        // Already stopped
      }
      this.musicSource = null;
    }
    this.musicGain = null;
  }
}
