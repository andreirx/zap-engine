// useZapEngine — React hook that encapsulates the full engine lifecycle.
//
// Manages: Worker, Renderer (WebGPU→Canvas2D fallback), SharedArrayBuffer reading,
// requestAnimationFrame render loop, input forwarding, resize, audio, and game events.
//
// Imported via '@zap/engine/react' — NOT part of core engine exports.

import { useEffect, useRef, useState, useCallback } from 'react';
import {
  initRenderer,
  loadManifest,
  loadAssetBlobs,
  HEADER_INSTANCE_COUNT,
  HEADER_ATLAS_SPLIT,
  HEADER_EFFECTS_VERTEX_COUNT,
  HEADER_SDF_INSTANCE_COUNT,
  HEADER_VECTOR_VERTEX_COUNT,
  HEADER_WORLD_WIDTH,
  HEADER_WORLD_HEIGHT,
  INSTANCE_FLOATS,
  EFFECTS_VERTEX_FLOATS,
  SDF_INSTANCE_FLOATS,
  VECTOR_VERTEX_FLOATS,
  ProtocolLayout,
  SoundManager,
} from '../index';
import type { Renderer, SoundConfig } from '../index';

/** Game event forwarded from the worker. */
export interface GameEvent {
  kind: number;
  a: number;
  b: number;
  c: number;
}

/** Configuration for the useZapEngine hook. */
export interface ZapEngineConfig {
  /** URL to the game's wasm-bindgen JS glue (e.g., '/pkg/my_game.js'). */
  wasmUrl: string;
  /** URL to the assets.json manifest. */
  assetsUrl: string;
  /** Base path for asset files (defaults to directory of assetsUrl). */
  assetBasePath?: string;
  /** Game world width in world units (default: 800). */
  gameWidth?: number;
  /** Game world height in world units (default: 600). */
  gameHeight?: number;
  /** Force Canvas 2D rendering — skip WebGPU (default: false). */
  force2D?: boolean;
  /** Callback for game events from the worker. */
  onGameEvent?: (events: GameEvent[]) => void;
  /** Sound configuration. If provided, audio is initialized on first interaction. */
  sounds?: SoundConfig;
}

/** Return value of the useZapEngine hook. */
export interface ZapEngineState {
  /** Ref to attach to a <canvas> element. */
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  /** Send a custom message to the worker. */
  sendEvent: (event: Record<string, unknown>) => void;
  /** Current frames per second (updated each frame). */
  fps: number;
  /** True once the worker and renderer are fully initialized. */
  isReady: boolean;
  /** Canvas key — use as React key prop to force canvas remount on WebGPU fallback. */
  canvasKey: number;
}

export function useZapEngine(config: ZapEngineConfig): ZapEngineState {
  const {
    wasmUrl,
    assetsUrl,
    assetBasePath,
    gameWidth = 800,
    gameHeight = 600,
    force2D = false,
    onGameEvent,
    sounds: soundConfig,
  } = config;

  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [fps, setFps] = useState(0);
  const [isReady, setIsReady] = useState(false);
  const [canvasKey, setCanvasKey] = useState(0);

  // Mutable refs for values accessed in the render loop / event handlers
  const workerRef = useRef<Worker | null>(null);
  const rendererRef = useRef<Renderer | null>(null);
  const layoutRef = useRef<ProtocolLayout | null>(null);
  const sharedF32Ref = useRef<Float32Array | null>(null);
  const rafIdRef = useRef<number>(0);
  const soundManagerRef = useRef<SoundManager | null>(null);
  const onGameEventRef = useRef(onGameEvent);
  const force2DRef = useRef(force2D);

  // Keep callback ref fresh
  onGameEventRef.current = onGameEvent;
  force2DRef.current = force2D;

  const sendEvent = useCallback((event: Record<string, unknown>) => {
    workerRef.current?.postMessage(event);
  }, []);

  useEffect(() => {
    let cancelled = false;
    let animFrameId = 0;
    let lastFrameTime = performance.now();
    let frameCount = 0;
    let fpsAccumulator = 0;

    async function start() {
      const canvas = canvasRef.current;
      if (!canvas) return;

      // Compute asset base path from manifest URL
      const basePath = assetBasePath ?? assetsUrl.substring(0, assetsUrl.lastIndexOf('/') + 1);

      // Load manifest and atlas blobs
      const manifest = await loadManifest(assetsUrl);
      if (cancelled) return;
      const atlasBlobs = await loadAssetBlobs(manifest, basePath);
      if (cancelled) return;

      // Create worker
      const worker = new Worker(
        new URL('../worker/engine.worker.ts', import.meta.url),
        { type: 'module' },
      );
      workerRef.current = worker;

      // Sound manager — pre-initialize so audio buffers are decoded before first interaction.
      // AudioContext starts suspended; resume() on pointerdown handles unsuspension.
      if (soundConfig) {
        const sm = new SoundManager(soundConfig);
        soundManagerRef.current = sm;
        sm.init().catch(err => console.warn('[useZapEngine] Sound init failed:', err));
      }

      worker.onmessage = async (e: MessageEvent) => {
        if (cancelled) return;
        const { type } = e.data;

        if (type === 'ready') {
          let layout: ProtocolLayout;
          let sharedF32: Float32Array | null = null;

          if (e.data.sharedBuffer) {
            sharedF32 = new Float32Array(e.data.sharedBuffer);
            layout = ProtocolLayout.fromHeader(sharedF32);
          } else {
            layout = new ProtocolLayout(
              e.data.maxInstances,
              e.data.maxEffectsVertices,
              e.data.maxSounds,
              e.data.maxEvents,
              e.data.maxSdfInstances,
              e.data.maxVectorVertices ?? 0,
            );
          }

          layoutRef.current = layout;
          sharedF32Ref.current = sharedF32;

          // Init renderer
          try {
            const renderer = await initRenderer({
              canvas,
              manifest,
              atlasBlobs,
              gameWidth,
              gameHeight,
              force2D: force2DRef.current,
              maxInstances: layout.maxInstances,
              maxEffectsVertices: layout.maxEffectsVertices,
              maxSdfInstances: layout.maxSdfInstances,
              maxVectorVertices: layout.maxVectorVertices,
            });
            if (cancelled) return;
            rendererRef.current = renderer;
            setIsReady(true);
            startRenderLoop();
          } catch (err) {
            if (err instanceof Error && err.message === 'WebGPUInitFailed') {
              // WebGPU tainted the canvas — remount with force2D
              worker.postMessage({ type: 'stop' });
              force2DRef.current = true;
              setCanvasKey(k => k + 1);
              return;
            }
            console.error('[useZapEngine] Renderer init failed:', err);
          }
        } else if (type === 'sound') {
          const sm = soundManagerRef.current;
          if (sm) {
            for (const id of e.data.events) {
              sm.play(id);
            }
          }
        } else if (type === 'event') {
          onGameEventRef.current?.(e.data.events);
        } else if (type === 'frame' && !sharedF32Ref.current) {
          // postMessage fallback
          const buf = new Float32Array(e.data.buffer);
          drawFromBuffer(buf);
        }
      };

      // Send init to worker (include manifest JSON for sprite registry)
      worker.postMessage({ type: 'init', wasmUrl, manifestJson: JSON.stringify(manifest) });

      // --- Input forwarding ---
      function onPointerDown(e: PointerEvent) {
        soundManagerRef.current?.resume();
        workerRef.current?.postMessage({
          type: 'pointer_down',
          x: e.offsetX,
          y: e.offsetY,
        });
      }
      function onPointerUp(e: PointerEvent) {
        workerRef.current?.postMessage({
          type: 'pointer_up',
          x: e.offsetX,
          y: e.offsetY,
        });
      }
      function onPointerMove(e: PointerEvent) {
        workerRef.current?.postMessage({
          type: 'pointer_move',
          x: e.offsetX,
          y: e.offsetY,
        });
      }
      function onKeyDown(e: KeyboardEvent) {
        workerRef.current?.postMessage({
          type: 'key_down',
          keyCode: e.keyCode,
        });
      }
      function onKeyUp(e: KeyboardEvent) {
        workerRef.current?.postMessage({
          type: 'key_up',
          keyCode: e.keyCode,
        });
      }

      canvas.addEventListener('pointerdown', onPointerDown);
      canvas.addEventListener('pointerup', onPointerUp);
      canvas.addEventListener('pointermove', onPointerMove);
      window.addEventListener('keydown', onKeyDown);
      window.addEventListener('keyup', onKeyUp);

      // --- Resize handling ---
      function handleResize() {
        const c = canvasRef.current;
        if (!c) return;
        c.width = c.clientWidth * devicePixelRatio;
        c.height = c.clientHeight * devicePixelRatio;
        rendererRef.current?.resize(c.width, c.height);
        // Send CSS dimensions to worker for coordinate conversion
        workerRef.current?.postMessage({
          type: 'resize',
          width: c.clientWidth,
          height: c.clientHeight,
        });
      }

      const resizeObserver = new ResizeObserver(handleResize);
      resizeObserver.observe(canvas);
      // Initial size
      handleResize();

      // --- Cleanup ---
      return () => {
        cancelled = true;
        canvas.removeEventListener('pointerdown', onPointerDown);
        canvas.removeEventListener('pointerup', onPointerUp);
        canvas.removeEventListener('pointermove', onPointerMove);
        window.removeEventListener('keydown', onKeyDown);
        window.removeEventListener('keyup', onKeyUp);
        resizeObserver.disconnect();
        cancelAnimationFrame(animFrameId);
        worker.postMessage({ type: 'stop' });
        worker.terminate();
        workerRef.current = null;
        rendererRef.current = null;
        layoutRef.current = null;
        sharedF32Ref.current = null;
        setIsReady(false);
      };
    }

    // --- Draw ---
    function drawFromBuffer(buf: Float32Array) {
      const renderer = rendererRef.current;
      const layout = layoutRef.current;
      if (!renderer || !layout) return;

      const instanceCount = buf[HEADER_INSTANCE_COUNT];
      const atlasSplit = buf[HEADER_ATLAS_SPLIT];
      const effectsVertexCount = buf[HEADER_EFFECTS_VERTEX_COUNT];
      const sdfInstanceCount = buf[HEADER_SDF_INSTANCE_COUNT];
      const vectorVertexCount = buf[HEADER_VECTOR_VERTEX_COUNT];

      if (instanceCount > 0 || sdfInstanceCount > 0 || vectorVertexCount > 0) {
        const instanceData = buf.subarray(
          layout.instanceDataOffset,
          layout.instanceDataOffset + instanceCount * INSTANCE_FLOATS,
        );

        let effectsData: Float32Array | undefined;
        if (effectsVertexCount > 0) {
          effectsData = buf.subarray(
            layout.effectsDataOffset,
            layout.effectsDataOffset + effectsVertexCount * EFFECTS_VERTEX_FLOATS,
          );
        }

        let sdfData: Float32Array | undefined;
        if (sdfInstanceCount > 0) {
          sdfData = buf.subarray(
            layout.sdfDataOffset,
            layout.sdfDataOffset + sdfInstanceCount * SDF_INSTANCE_FLOATS,
          );
        }

        let vectorData: Float32Array | undefined;
        if (vectorVertexCount > 0) {
          vectorData = buf.subarray(
            layout.vectorDataOffset,
            layout.vectorDataOffset + vectorVertexCount * VECTOR_VERTEX_FLOATS,
          );
        }

        renderer.draw(instanceData, instanceCount, atlasSplit, effectsData, effectsVertexCount, sdfData, sdfInstanceCount, vectorData, vectorVertexCount);
      }
    }

    // --- Render loop ---
    function startRenderLoop() {
      function frame() {
        const buf = sharedF32Ref.current;
        if (buf) {
          drawFromBuffer(buf);
        }

        // FPS calculation
        const now = performance.now();
        frameCount++;
        fpsAccumulator += now - lastFrameTime;
        lastFrameTime = now;
        if (fpsAccumulator >= 1000) {
          setFps(Math.round(frameCount * 1000 / fpsAccumulator));
          frameCount = 0;
          fpsAccumulator = 0;
        }

        animFrameId = requestAnimationFrame(frame);
      }
      animFrameId = requestAnimationFrame(frame);
      rafIdRef.current = animFrameId;
    }

    let cleanup: (() => void) | undefined;
    start().then(fn => {
      cleanup = fn;
    });

    return () => {
      cancelled = true;
      cancelAnimationFrame(animFrameId);
      cleanup?.();
    };
  // Re-run effect when canvasKey changes (WebGPU fallback remount)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wasmUrl, assetsUrl, assetBasePath, gameWidth, gameHeight, canvasKey]);

  return { canvasRef, sendEvent, fps, isReady, canvasKey };
}
