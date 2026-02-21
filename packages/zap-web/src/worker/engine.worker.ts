// Generic engine worker — loads a game WASM module and runs the simulation loop.
// Accepts a wasmUrl via 'init' message, dynamically imports it.
// Communicates via SharedArrayBuffer (preferred) or postMessage fallback.

import {
  HEADER_FLOATS,
  HEADER_FRAME_COUNTER,
  HEADER_MAX_INSTANCES,
  HEADER_INSTANCE_COUNT,
  HEADER_ATLAS_SPLIT,
  HEADER_MAX_EFFECTS_VERTICES,
  HEADER_EFFECTS_VERTEX_COUNT,
  HEADER_WORLD_WIDTH,
  HEADER_WORLD_HEIGHT,
  HEADER_MAX_SOUNDS,
  HEADER_SOUND_COUNT,
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
  HEADER_WASM_TIME_US,
  PROTOCOL_VERSION,
  INSTANCE_FLOATS,
  EFFECTS_VERTEX_FLOATS,
  EVENT_FLOATS,
  SDF_INSTANCE_FLOATS,
  VECTOR_VERTEX_FLOATS,
  LAYER_BATCH_FLOATS,
  LIGHT_FLOATS,
  ProtocolLayout,
} from './protocol';
import { computeProjection } from '../renderer/camera';

// Standard WASM export names expected from any game module
interface GameWasmExports {
  game_init: () => void;
  game_tick: (dt: number) => void;
  game_pointer_down: (x: number, y: number) => void;
  game_pointer_up: (x: number, y: number) => void;
  game_pointer_move: (x: number, y: number) => void;
  game_key_down: (keyCode: number) => void;
  game_key_up: (keyCode: number) => void;
  get_instances_ptr: () => number;
  get_instance_count: () => number;
  get_effects_ptr: () => number;
  get_effects_vertex_count: () => number;
  get_sound_events_ptr: () => number;
  get_sound_events_len: () => number;
  get_game_events_ptr: () => number;
  get_game_events_len: () => number;
  get_world_width: () => number;
  get_world_height: () => number;
  get_atlas_split: () => number;
  get_max_instances: () => number;
  get_max_effects_vertices: () => number;
  get_max_sounds: () => number;
  get_max_events: () => number;
  get_buffer_total_floats: () => number;
  get_sdf_instances_ptr: () => number;
  get_sdf_instance_count: () => number;
  get_max_sdf_instances: () => number;
  game_custom_event?: (kind: number, a: number, b: number, c: number) => void;
  load_level?: (json: string) => void;
  reload_scripts?: (json: string) => void;
  reload_game_manifest?: (json: string) => void;
  reload_sprite_manifest?: (json: string) => void;
  game_load_manifest?: (json: string) => void;
  // Optional vector exports (feature-gated in Rust)
  get_vector_vertices_ptr?: () => number;
  get_vector_vertex_count?: () => number;
  get_max_vector_vertices?: () => number;
  // Layer batch exports
  get_layer_batches_ptr?: () => number;
  get_layer_batch_count?: () => number;
  get_max_layer_batches?: () => number;
  get_layer_batch_data_offset?: () => number;
  // Bake state export
  get_bake_state?: () => number;
  // Lighting exports
  get_lights_ptr?: () => number;
  get_light_count?: () => number;
  get_max_lights?: () => number;
  get_ambient_r?: () => number;
  get_ambient_g?: () => number;
  get_ambient_b?: () => number;
}

const HAS_SAB = typeof SharedArrayBuffer !== 'undefined';

let sharedBuffer: SharedArrayBuffer | null = null;
let sharedF32: Float32Array | null = null;
let sharedI32: Int32Array | null = null;
let running = false;
let wasmMemory: WebAssembly.Memory | null = null;
let wasm: GameWasmExports | null = null;
let layout: ProtocolLayout | null = null;

// Canvas CSS dimensions for coordinate conversion (set via 'resize' message)
let canvasWidth = 0;
let canvasHeight = 0;
let worldWidth = 0;
let worldHeight = 0;

/** Convert canvas CSS pixel coordinates to world coordinates. */
function screenToWorld(cssX: number, cssY: number): { x: number; y: number } {
  if (canvasWidth <= 0 || canvasHeight <= 0) {
    return { x: cssX, y: cssY };
  }
  const { projWidth, projHeight } = computeProjection(canvasWidth, canvasHeight, worldWidth, worldHeight);
  return {
    x: cssX * projWidth / canvasWidth,
    y: cssY * projHeight / canvasHeight,
  };
}

async function initialize(wasmUrl: string, manifestJson?: string) {
  // Dynamic import of the WASM module
  const mod = await import(/* @vite-ignore */ wasmUrl);
  const initResult = await mod.default();
  wasmMemory = initResult.memory;

  // Collect exports
  wasm = {
    game_init: mod.game_init,
    game_tick: mod.game_tick,
    game_pointer_down: mod.game_pointer_down,
    game_pointer_up: mod.game_pointer_up,
    game_pointer_move: mod.game_pointer_move,
    game_key_down: mod.game_key_down,
    game_key_up: mod.game_key_up,
    get_instances_ptr: mod.get_instances_ptr,
    get_instance_count: mod.get_instance_count,
    get_effects_ptr: mod.get_effects_ptr,
    get_effects_vertex_count: mod.get_effects_vertex_count,
    get_sound_events_ptr: mod.get_sound_events_ptr,
    get_sound_events_len: mod.get_sound_events_len,
    get_game_events_ptr: mod.get_game_events_ptr,
    get_game_events_len: mod.get_game_events_len,
    get_world_width: mod.get_world_width,
    get_world_height: mod.get_world_height,
    get_atlas_split: mod.get_atlas_split,
    get_max_instances: mod.get_max_instances,
    get_max_effects_vertices: mod.get_max_effects_vertices,
    get_max_sounds: mod.get_max_sounds,
    get_max_events: mod.get_max_events,
    get_buffer_total_floats: mod.get_buffer_total_floats,
    get_sdf_instances_ptr: mod.get_sdf_instances_ptr,
    get_sdf_instance_count: mod.get_sdf_instance_count,
    get_max_sdf_instances: mod.get_max_sdf_instances,
    game_custom_event: mod.game_custom_event,
    game_load_manifest: mod.game_load_manifest,
    // Optional vector exports (feature-gated)
    get_vector_vertices_ptr: mod.get_vector_vertices_ptr,
    get_vector_vertex_count: mod.get_vector_vertex_count,
    get_max_vector_vertices: mod.get_max_vector_vertices,
    // Layer batch exports
    get_layer_batches_ptr: mod.get_layer_batches_ptr,
    get_layer_batch_count: mod.get_layer_batch_count,
    get_max_layer_batches: mod.get_max_layer_batches,
    get_layer_batch_data_offset: mod.get_layer_batch_data_offset,
    // Bake state export
    get_bake_state: mod.get_bake_state,
    // Lighting exports
    get_lights_ptr: mod.get_lights_ptr,
    get_light_count: mod.get_light_count,
    get_max_lights: mod.get_max_lights,
    get_ambient_r: mod.get_ambient_r,
    get_ambient_g: mod.get_ambient_g,
    get_ambient_b: mod.get_ambient_b,
    // Game-specific exports for level/script/manifest loading
    load_level: mod.load_level,
    reload_scripts: mod.reload_scripts,
    reload_game_manifest: mod.reload_game_manifest,
    reload_sprite_manifest: mod.reload_sprite_manifest,
  };

  wasm.game_init();

  // Load manifest into WASM sprite registry (if available)
  if (manifestJson && wasm.game_load_manifest) {
    wasm.game_load_manifest(manifestJson);
  }

  // Capture world dimensions for coordinate conversion
  worldWidth = wasm.get_world_width();
  worldHeight = wasm.get_world_height();

  // Send initial viewport dimensions if canvas size is already known
  // and process immediately with a zero-dt tick so the game has correct
  // dimensions before the first render
  if (canvasWidth > 0 && canvasHeight > 0) {
    const proj = computeProjection(canvasWidth, canvasHeight, worldWidth, worldHeight);
    wasm.game_custom_event?.(99, proj.projWidth, proj.projHeight, 0);
    wasm.game_tick(0); // Process resize event before first render
  }

  // Build layout from WASM-reported capacities
  layout = ProtocolLayout.fromWasm(wasm);

  if (HAS_SAB) {
    sharedBuffer = new SharedArrayBuffer(layout.bufferTotalBytes);
    sharedF32 = new Float32Array(sharedBuffer);
    sharedI32 = new Int32Array(sharedBuffer);

    // Write capacities and protocol version into header (once)
    sharedF32[HEADER_MAX_INSTANCES] = layout.maxInstances;
    sharedF32[HEADER_MAX_EFFECTS_VERTICES] = layout.maxEffectsVertices;
    sharedF32[HEADER_MAX_SOUNDS] = layout.maxSounds;
    sharedF32[HEADER_MAX_EVENTS] = layout.maxEvents;
    sharedF32[HEADER_PROTOCOL_VERSION] = PROTOCOL_VERSION;
    sharedF32[HEADER_MAX_SDF_INSTANCES] = layout.maxSdfInstances;
    sharedF32[HEADER_MAX_VECTOR_VERTICES] = layout.maxVectorVertices;
    sharedF32[HEADER_MAX_LAYER_BATCHES] = layout.maxLayerBatches;
    sharedF32[HEADER_LAYER_BATCH_OFFSET] = layout.layerBatchDataOffset;
    sharedF32[HEADER_MAX_LIGHTS] = layout.maxLights;

    self.postMessage({ type: 'ready', sharedBuffer, worldWidth, worldHeight });
  } else {
    console.warn('[worker] SharedArrayBuffer unavailable, using postMessage fallback');
    const buf = new ArrayBuffer(layout.bufferTotalBytes);
    sharedF32 = new Float32Array(buf);
    sharedI32 = new Int32Array(buf);

    // Write capacities into the fallback buffer too
    sharedF32[HEADER_MAX_INSTANCES] = layout.maxInstances;
    sharedF32[HEADER_MAX_EFFECTS_VERTICES] = layout.maxEffectsVertices;
    sharedF32[HEADER_MAX_SOUNDS] = layout.maxSounds;
    sharedF32[HEADER_MAX_EVENTS] = layout.maxEvents;
    sharedF32[HEADER_PROTOCOL_VERSION] = PROTOCOL_VERSION;
    sharedF32[HEADER_MAX_SDF_INSTANCES] = layout.maxSdfInstances;
    sharedF32[HEADER_MAX_VECTOR_VERTICES] = layout.maxVectorVertices;
    sharedF32[HEADER_MAX_LAYER_BATCHES] = layout.maxLayerBatches;
    sharedF32[HEADER_LAYER_BATCH_OFFSET] = layout.layerBatchDataOffset;
    sharedF32[HEADER_MAX_LIGHTS] = layout.maxLights;

    self.postMessage({
      type: 'ready',
      maxInstances: layout.maxInstances,
      maxEffectsVertices: layout.maxEffectsVertices,
      maxSounds: layout.maxSounds,
      maxEvents: layout.maxEvents,
      maxSdfInstances: layout.maxSdfInstances,
      maxVectorVertices: layout.maxVectorVertices,
      maxLayerBatches: layout.maxLayerBatches,
      maxLights: layout.maxLights,
      worldWidth,
      worldHeight,
    });
  }
}

function gameLoop() {
  if (!running || !sharedF32 || !sharedI32 || !wasmMemory || !wasm || !layout) return;

  try {
    const dt = 1.0 / 60.0;
    const wasmStart = performance.now();
    wasm.game_tick(dt);
    const wasmTimeUs = (performance.now() - wasmStart) * 1000; // Convert ms to μs

    const instanceCount = Math.min(wasm.get_instance_count(), layout.maxInstances);
    const effectsVertexCount = Math.min(wasm.get_effects_vertex_count(), layout.maxEffectsVertices);
    const soundLen = Math.min(wasm.get_sound_events_len(), layout.maxSounds);
    const eventLen = Math.min(wasm.get_game_events_len(), layout.maxEvents);
    const sdfCount = Math.min(wasm.get_sdf_instance_count(), layout.maxSdfInstances);
    const vectorVertexCount = wasm.get_vector_vertex_count
      ? Math.min(wasm.get_vector_vertex_count(), layout.maxVectorVertices)
      : 0;
    const layerBatchCount = wasm.get_layer_batch_count
      ? Math.min(wasm.get_layer_batch_count(), layout.maxLayerBatches)
      : 0;
    const lightCount = wasm.get_light_count
      ? Math.min(wasm.get_light_count(), layout.maxLights)
      : 0;

    // Write header
    sharedF32[HEADER_FRAME_COUNTER] += 1;
    sharedF32[HEADER_INSTANCE_COUNT] = instanceCount;
    sharedF32[HEADER_ATLAS_SPLIT] = wasm.get_atlas_split();
    sharedF32[HEADER_EFFECTS_VERTEX_COUNT] = effectsVertexCount;
    sharedF32[HEADER_WORLD_WIDTH] = wasm.get_world_width();
    sharedF32[HEADER_WORLD_HEIGHT] = wasm.get_world_height();
    sharedF32[HEADER_SOUND_COUNT] = soundLen;
    sharedF32[HEADER_EVENT_COUNT] = eventLen;
    sharedF32[HEADER_SDF_INSTANCE_COUNT] = sdfCount;
    sharedF32[HEADER_VECTOR_VERTEX_COUNT] = vectorVertexCount;
    sharedF32[HEADER_LAYER_BATCH_COUNT] = layerBatchCount;
    sharedF32[HEADER_BAKE_STATE] = wasm.get_bake_state?.() ?? 0;
    sharedF32[HEADER_LIGHT_COUNT] = lightCount;
    sharedF32[HEADER_AMBIENT_R] = wasm.get_ambient_r?.() ?? 1.0;
    sharedF32[HEADER_AMBIENT_G] = wasm.get_ambient_g?.() ?? 1.0;
    sharedF32[HEADER_AMBIENT_B] = wasm.get_ambient_b?.() ?? 1.0;
    sharedF32[HEADER_WASM_TIME_US] = wasmTimeUs;

    // Copy instance data
    if (instanceCount > 0) {
      const ptr = wasm.get_instances_ptr();
      const wasmData = new Float32Array(wasmMemory.buffer, ptr, instanceCount * INSTANCE_FLOATS);
      sharedF32.set(wasmData, layout.instanceDataOffset);
    }

    // Copy effects data
    if (effectsVertexCount > 0) {
      const ptr = wasm.get_effects_ptr();
      const effectsData = new Float32Array(wasmMemory.buffer, ptr, effectsVertexCount * EFFECTS_VERTEX_FLOATS);
      sharedF32.set(effectsData, layout.effectsDataOffset);
    }

    // Copy SDF instance data
    if (sdfCount > 0) {
      const ptr = wasm.get_sdf_instances_ptr();
      const sdfData = new Float32Array(wasmMemory.buffer, ptr, sdfCount * SDF_INSTANCE_FLOATS);
      sharedF32.set(sdfData, layout.sdfDataOffset);
    }

    // Copy vector vertex data
    if (vectorVertexCount > 0 && wasm.get_vector_vertices_ptr) {
      const ptr = wasm.get_vector_vertices_ptr();
      const vectorData = new Float32Array(wasmMemory.buffer, ptr, vectorVertexCount * VECTOR_VERTEX_FLOATS);
      sharedF32.set(vectorData, layout.vectorDataOffset);
    }

    // Copy layer batch data
    if (layerBatchCount > 0 && wasm.get_layer_batches_ptr) {
      const ptr = wasm.get_layer_batches_ptr();
      const batchData = new Float32Array(wasmMemory.buffer, ptr, layerBatchCount * LAYER_BATCH_FLOATS);
      sharedF32.set(batchData, layout.layerBatchDataOffset);
    }

    // Copy light data
    if (lightCount > 0 && wasm.get_lights_ptr) {
      const ptr = wasm.get_lights_ptr();
      const lightData = new Float32Array(wasmMemory.buffer, ptr, lightCount * LIGHT_FLOATS);
      sharedF32.set(lightData, layout.lightDataOffset);
    }

    // Forward sound events
    if (soundLen > 0) {
      const ptr = wasm.get_sound_events_ptr();
      const soundData = new Uint8Array(wasmMemory.buffer, ptr, soundLen);
      const events = Array.from(soundData);
      self.postMessage({ type: 'sound', events });
    }

    // Forward game events
    if (eventLen > 0) {
      const ptr = wasm.get_game_events_ptr();
      const eventData = new Float32Array(wasmMemory.buffer, ptr, eventLen * EVENT_FLOATS);
      const events = [];
      for (let i = 0; i < eventLen; i++) {
        events.push({
          kind: eventData[i * EVENT_FLOATS],
          a: eventData[i * EVENT_FLOATS + 1],
          b: eventData[i * EVENT_FLOATS + 2],
          c: eventData[i * EVENT_FLOATS + 3],
        });
      }
      self.postMessage({ type: 'event', events });
    }

    if (HAS_SAB) {
      Atomics.store(sharedI32!, 0, 1);
      Atomics.notify(sharedI32!, 0);
    } else {
      // Send frame data copy (only the used portion)
      const usedFloats = HEADER_FLOATS + layout.instanceDataFloats
        + effectsVertexCount * EFFECTS_VERTEX_FLOATS;
      self.postMessage({ type: 'frame', buffer: sharedF32!.buffer.slice(0, usedFloats * 4) });
    }
  } catch (err) {
    console.error('[worker] gameLoop error:', err);
    running = false;
    return;
  }

  if (running) {
    setTimeout(gameLoop, 16);
  }
}

self.onmessage = (e: MessageEvent) => {
  const { type } = e.data;

  switch (type) {
    case 'init':
      initialize(e.data.wasmUrl, e.data.manifestJson).then(() => {
        running = true;
        gameLoop();
      });
      break;

    case 'pointer_down': {
      const w = screenToWorld(e.data.x, e.data.y);
      wasm?.game_pointer_down(w.x, w.y);
      break;
    }

    case 'pointer_up': {
      const w = screenToWorld(e.data.x, e.data.y);
      wasm?.game_pointer_up(w.x, w.y);
      break;
    }

    case 'pointer_move': {
      const w = screenToWorld(e.data.x, e.data.y);
      wasm?.game_pointer_move(w.x, w.y);
      break;
    }

    case 'resize':
      canvasWidth = e.data.width;
      canvasHeight = e.data.height;
      // Forward visible world dimensions to game so it can adapt layout
      if (worldWidth > 0 && worldHeight > 0) {
        const proj = computeProjection(canvasWidth, canvasHeight, worldWidth, worldHeight);
        wasm?.game_custom_event?.(99, proj.projWidth, proj.projHeight, 0);
      }
      break;

    case 'key_down':
      wasm?.game_key_down(e.data.keyCode);
      break;

    case 'key_up':
      wasm?.game_key_up(e.data.keyCode);
      break;

    case 'custom':
      wasm?.game_custom_event?.(e.data.kind ?? 0, e.data.a ?? 0, e.data.b ?? 0, e.data.c ?? 0);
      break;

    case 'load_level':
      if (wasm?.load_level && e.data.json) {
        wasm.load_level(e.data.json);
      }
      break;

    case 'reload_scripts':
      if (wasm?.reload_scripts && e.data.json) {
        wasm.reload_scripts(e.data.json);
      }
      break;

    case 'reload_game_manifest':
      if (wasm?.reload_game_manifest && e.data.json) {
        wasm.reload_game_manifest(e.data.json);
      }
      break;

    case 'reload_sprite_manifest':
      if (wasm?.reload_sprite_manifest && e.data.json) {
        wasm.reload_sprite_manifest(e.data.json);
      }
      break;

    case 'stop':
      running = false;
      break;

    case 'resume':
      if (!running && sharedF32 && sharedI32 && wasmMemory) {
        running = true;
        gameLoop();
      }
      break;
  }
};
