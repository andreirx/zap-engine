// Sound manager using Web Audio API.
// Configurable — accepts a sound map instead of hardcoded names.

export interface SoundConfig {
  /** Map of event ID → audio file path. */
  sounds: Record<number, string>;
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

  async init(): Promise<void> {
    this.ctx = new AudioContext();
    const basePath = this.config.basePath ?? '/audio/';

    const loads: Promise<void>[] = [];

    // Load all sound effects
    for (const path of Object.values(this.config.sounds)) {
      loads.push(this.loadSound(basePath + path));
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
      console.warn(`Failed to load sound: ${fullPath}`);
    }
  }

  async resume(): Promise<void> {
    if (this.ctx?.state === 'suspended') {
      await this.ctx.resume();
    }
  }

  /** Play a sound event by its numeric ID. */
  play(eventId: number): void {
    const path = this.config.sounds[eventId];
    if (!path) return;
    const basePath = this.config.basePath ?? '/audio/';
    this.playPath(basePath + path);
  }

  /** Play a sound by full path. */
  private playPath(fullPath: string): void {
    if (!this.ctx || !this.loaded) return;
    const buffer = this.buffers.get(fullPath);
    if (!buffer) return;

    const source = this.ctx.createBufferSource();
    source.buffer = buffer;
    source.connect(this.ctx.destination);
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
