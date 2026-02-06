// WebGPU renderer — reads simulation state from SharedArrayBuffer and draws.
// Configures rgba16float + display-p3 + extended tone mapping for HDR/EDR.
// Manifest-driven: accepts N atlases, creates one pipeline per atlas.

import shaderSource from './shaders.wgsl?raw';
import sdfShaderSource from './molecule.wgsl?raw';
import vectorShaderSource from './vector.wgsl?raw';
import lightingShaderSource from './lighting.wgsl?raw';
import { buildProjectionMatrix, computeProjection } from './camera';
import { packColorsForGPU } from './constants';
import type { Renderer, RenderTier, LayerBatchDescriptor, BakeState, LightingState } from './types';
import type { AssetManifest, GPUTextureAsset } from '../assets/manifest';
import { createGPUTextureFromBlob } from '../assets/loader';
import { LayerCompositor } from './compositor';

// Bytes per RenderInstance: 8 × f32 = 32 bytes
const INSTANCE_STRIDE = 32;
// Effects vertex: 5 floats = 20 bytes
const EFFECTS_VERTEX_FLOATS = 5;
const EFFECTS_VERTEX_BYTES = EFFECTS_VERTEX_FLOATS * 4;

// SDF instance: 12 floats = 48 bytes
const SDF_INSTANCE_FLOATS = 12;
const SDF_INSTANCE_STRIDE = SDF_INSTANCE_FLOATS * 4;

// Vector vertex: 6 floats = 24 bytes (x, y, r, g, b, a)
const VECTOR_VERTEX_FLOATS = 6;
const VECTOR_VERTEX_BYTES = VECTOR_VERTEX_FLOATS * 4;

// Default capacities (matching GameConfig::default())
const DEFAULT_MAX_INSTANCES = 512;
const DEFAULT_MAX_EFFECTS_VERTICES = 16384;
const DEFAULT_MAX_SDF_INSTANCES = 128;
const DEFAULT_MAX_VECTOR_VERTICES = 16384;

export interface WebGPURendererConfig {
  canvas: HTMLCanvasElement;
  manifest: AssetManifest;
  atlasBlobs: Map<string, Blob>;
  /** Optional normal map blobs (atlas name → Blob) for per-pixel lighting. */
  normalMapBlobs?: Map<string, Blob>;
  gameWidth: number;
  gameHeight: number;
  /** Max render instances for GPU buffer allocation (default: 512). */
  maxInstances?: number;
  /** Max effects vertices for GPU buffer allocation (default: 16384). */
  maxEffectsVertices?: number;
  /** Max SDF instances for GPU buffer allocation (default: 128). */
  maxSdfInstances?: number;
  /** Max vector vertices for GPU buffer allocation (default: 16384). */
  maxVectorVertices?: number;
}

export async function initWebGPURenderer(config: WebGPURendererConfig): Promise<Renderer> {
  const {
    canvas,
    manifest,
    atlasBlobs,
    normalMapBlobs,
    gameWidth,
    gameHeight,
    maxInstances = DEFAULT_MAX_INSTANCES,
    maxEffectsVertices = DEFAULT_MAX_EFFECTS_VERTICES,
    maxSdfInstances = DEFAULT_MAX_SDF_INSTANCES,
    maxVectorVertices = DEFAULT_MAX_VECTOR_VERTICES,
  } = config;

  // ---- GPU Init ----
  if (!navigator.gpu) {
    throw new Error('WebGPU not supported');
  }

  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) {
    throw new Error('No WebGPU adapter found');
  }

  const device = await adapter.requestDevice();
  const context = canvas.getContext('webgpu');
  if (!context) {
    throw new Error('Failed to get WebGPU context');
  }

  // Progressive configure — try full HDR/EDR, then basic rgba16float, then preferred format.
  let format: GPUTextureFormat = 'rgba16float';
  let tier: RenderTier = 'sdr';

  try {
    context.configure({
      device,
      format: 'rgba16float',
      colorSpace: 'display-p3',
      toneMapping: { mode: 'extended' },
      alphaMode: 'premultiplied',
    });
    tier = 'hdr-edr';
  } catch {
    try {
      context.configure({
        device,
        format: 'rgba16float',
        alphaMode: 'premultiplied',
      });
      tier = 'hdr-srgb';
    } catch {
      format = navigator.gpu.getPreferredCanvasFormat();
      context.configure({
        device,
        format,
        alphaMode: 'premultiplied',
      });
      tier = 'sdr';
    }
  }

  // Per-tier glow multipliers for shader override constants.
  const GLOW_MULT: Record<Exclude<RenderTier, 'canvas2d'>, { effects: number; sdf: number }> = {
    'hdr-edr':  { effects: 6.4, sdf: 5.4 },
    'hdr-srgb': { effects: 3.0, sdf: 2.5 },
    'sdr':      { effects: 1.0, sdf: 0.5 },
  };

  console.info(`[renderer] WebGPU tier: ${tier} (format: ${format})`);

  // ---- Load textures from manifest ----
  const textures: GPUTextureAsset[] = [];
  for (const atlas of manifest.atlases) {
    const blob = atlasBlobs.get(atlas.name);
    if (!blob) {
      throw new Error(`Missing blob for atlas: ${atlas.name}`);
    }
    textures.push(await createGPUTextureFromBlob(device, blob));
  }

  // ---- Load normal map textures (optional, per-atlas) ----
  // normalTextures[i] is the normal map for atlas i, or null if none.
  // Normal maps are loaded WITHOUT premultiplied alpha to preserve raw normal values.
  const normalTextures: (GPUTextureAsset | null)[] = [];
  for (const atlas of manifest.atlases) {
    const normalBlob = normalMapBlobs?.get(atlas.name);
    if (normalBlob) {
      normalTextures.push(await createGPUTextureFromBlob(device, normalBlob, false));
    } else {
      normalTextures.push(null);
    }
  }
  const hasNormalMaps = normalTextures.some((t) => t !== null);

  // ---- Shader Module ----
  const shaderModule = device.createShaderModule({ code: shaderSource });

  // ---- Camera Uniform ----
  const cameraBuffer = device.createBuffer({
    size: 64,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const cameraBindGroupLayout = device.createBindGroupLayout({
    entries: [{
      binding: 0,
      visibility: GPUShaderStage.VERTEX,
      buffer: { type: 'uniform' },
    }],
  });

  const cameraBindGroup = device.createBindGroup({
    layout: cameraBindGroupLayout,
    entries: [{ binding: 0, resource: { buffer: cameraBuffer } }],
  });

  // ---- Segment Colors UBO ----
  const colorsData = packColorsForGPU();
  const colorsBuffer = device.createBuffer({
    size: colorsData.byteLength,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(colorsBuffer, 0, colorsData);

  const colorsBindGroupLayout = device.createBindGroupLayout({
    entries: [{
      binding: 0,
      visibility: GPUShaderStage.FRAGMENT,
      buffer: { type: 'uniform' },
    }],
  });

  const colorsBindGroup = device.createBindGroup({
    layout: colorsBindGroupLayout,
    entries: [{ binding: 0, resource: { buffer: colorsBuffer } }],
  });

  // ---- Texture Bind Group Layout ----
  const textureBindGroupLayout = device.createBindGroupLayout({
    entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
    ],
  });

  const sampler = device.createSampler({
    magFilter: 'linear',
    minFilter: 'linear',
    mipmapFilter: 'linear',
  });

  // Create a bind group per atlas
  const textureBindGroups: GPUBindGroup[] = textures.map((tex) =>
    device.createBindGroup({
      layout: textureBindGroupLayout,
      entries: [
        { binding: 0, resource: tex.view },
        { binding: 1, resource: sampler },
      ],
    })
  );

  // Fallback 1×1 white texture for effects when no atlases exist
  const fallbackTex = device.createTexture({
    size: [1, 1],
    format: 'rgba8unorm',
    usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
  });
  device.queue.writeTexture(
    { texture: fallbackTex },
    new Uint8Array([255, 255, 255, 255]),
    { bytesPerRow: 4 },
    [1, 1],
  );
  const fallbackTextureBindGroup = device.createBindGroup({
    layout: textureBindGroupLayout,
    entries: [
      { binding: 0, resource: fallbackTex.createView() },
      { binding: 1, resource: sampler },
    ],
  });

  // ---- Instance Storage Buffer ----
  const instanceBuffer = device.createBuffer({
    size: INSTANCE_STRIDE * maxInstances,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });

  const instanceBindGroupLayout = device.createBindGroupLayout({
    entries: [{
      binding: 0,
      visibility: GPUShaderStage.VERTEX,
      buffer: { type: 'read-only-storage' },
    }],
  });

  const instanceBindGroup = device.createBindGroup({
    layout: instanceBindGroupLayout,
    entries: [{ binding: 0, resource: { buffer: instanceBuffer } }],
  });

  // ---- Effects Vertex Buffer ----
  const effectsBuffer = device.createBuffer({
    size: EFFECTS_VERTEX_BYTES * maxEffectsVertices,
    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  });

  // ---- Vector Vertex Buffer ----
  const vectorBuffer = device.createBuffer({
    size: VECTOR_VERTEX_BYTES * maxVectorVertices,
    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  });

  // ---- Pipeline Layouts ----
  const tilePipelineLayout = device.createPipelineLayout({
    bindGroupLayouts: [cameraBindGroupLayout, textureBindGroupLayout, instanceBindGroupLayout],
  });

  const emptyBindGroupLayout = device.createBindGroupLayout({ entries: [] });
  const emptyBindGroup = device.createBindGroup({ layout: emptyBindGroupLayout, entries: [] });

  const effectsPipelineLayout = device.createPipelineLayout({
    bindGroupLayouts: [cameraBindGroupLayout, textureBindGroupLayout, emptyBindGroupLayout, colorsBindGroupLayout],
  });

  // Alpha blend targets
  const alphaBlendTargets: GPUColorTargetState[] = [{
    format,
    blend: {
      color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
      alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
    },
  }];

  // ---- Create one alpha pipeline per atlas ----
  const alphaPipelines: GPURenderPipeline[] = manifest.atlases.map((atlas) =>
    device.createRenderPipeline({
      layout: tilePipelineLayout,
      vertex: {
        module: shaderModule,
        entryPoint: 'vs_main',
        constants: { ATLAS_COLS: atlas.cols, ATLAS_ROWS: atlas.rows },
      },
      fragment: {
        module: shaderModule,
        entryPoint: 'fs_main',
        targets: alphaBlendTargets,
      },
      primitive: { topology: 'triangle-list' },
    })
  );

  // ---- Additive Pipeline (effects) ----
  const additivePipeline = device.createRenderPipeline({
    layout: effectsPipelineLayout,
    vertex: {
      module: shaderModule,
      entryPoint: 'vs_effects',
      buffers: [{
        arrayStride: EFFECTS_VERTEX_BYTES,
        attributes: [
          { shaderLocation: 0, offset: 0, format: 'float32x3' },
          { shaderLocation: 1, offset: 12, format: 'float32x2' },
        ],
      }],
    },
    fragment: {
      module: shaderModule,
      entryPoint: 'fs_additive',
      constants: { EFFECTS_HDR_MULT: GLOW_MULT[tier as Exclude<RenderTier, 'canvas2d'>].effects },
      targets: [{
        format,
        blend: {
          color: { srcFactor: 'src-alpha', dstFactor: 'one', operation: 'add' },
          alpha: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
        },
      }],
    },
    primitive: { topology: 'triangle-list' },
  });

  // ---- SDF Pipeline (molecule rendering) ----
  const sdfShaderModule = device.createShaderModule({ code: sdfShaderSource });

  const sdfStorageBuffer = device.createBuffer({
    size: SDF_INSTANCE_STRIDE * maxSdfInstances,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });

  const sdfBindGroupLayout = device.createBindGroupLayout({
    entries: [{
      binding: 0,
      visibility: GPUShaderStage.VERTEX,
      buffer: { type: 'read-only-storage' },
    }],
  });

  const sdfBindGroup = device.createBindGroup({
    layout: sdfBindGroupLayout,
    entries: [{ binding: 0, resource: { buffer: sdfStorageBuffer } }],
  });

  const sdfPipelineLayout = device.createPipelineLayout({
    bindGroupLayouts: [cameraBindGroupLayout, sdfBindGroupLayout],
  });

  const sdfPipeline = device.createRenderPipeline({
    layout: sdfPipelineLayout,
    vertex: {
      module: sdfShaderModule,
      entryPoint: 'vs_sdf',
    },
    fragment: {
      module: sdfShaderModule,
      entryPoint: 'fs_sdf',
      constants: { SDF_EMISSIVE_MULT: GLOW_MULT[tier as Exclude<RenderTier, 'canvas2d'>].sdf },
      targets: [{
        format,
        blend: {
          color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
          alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
        },
      }],
    },
    primitive: { topology: 'triangle-list' },
  });

  // ---- Vector Pipeline (Lyon-tessellated geometry) ----
  const vectorShaderModule = device.createShaderModule({ code: vectorShaderSource });

  const vectorPipelineLayout = device.createPipelineLayout({
    bindGroupLayouts: [cameraBindGroupLayout],
  });

  const vectorPipeline = device.createRenderPipeline({
    layout: vectorPipelineLayout,
    vertex: {
      module: vectorShaderModule,
      entryPoint: 'vs_vector',
      buffers: [{
        arrayStride: VECTOR_VERTEX_BYTES,
        attributes: [
          { shaderLocation: 0, offset: 0, format: 'float32x2' },   // position
          { shaderLocation: 1, offset: 8, format: 'float32x4' },   // color
        ],
      }],
    },
    fragment: {
      module: vectorShaderModule,
      entryPoint: 'fs_vector',
      constants: { VECTOR_HDR_MULT: GLOW_MULT[tier as Exclude<RenderTier, 'canvas2d'>].effects },
      targets: alphaBlendTargets,
    },
    primitive: { topology: 'triangle-list' },
  });

  // ---- Flat normal placeholder (1×1 RGBA: 128,128,255,255 → tangent-space (0,0,1)) ----
  const flatNormalTexture = device.createTexture({
    size: { width: 1, height: 1 },
    format: 'rgba8unorm',
    usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
  });
  device.queue.writeTexture(
    { texture: flatNormalTexture },
    new Uint8Array([128, 128, 255, 255]),
    { bytesPerRow: 4 },
    { width: 1, height: 1 },
  );
  const flatNormalView = flatNormalTexture.createView();

  // Resolve the "active" normal map view per atlas: real normal or flat fallback
  const normalMapViews: GPUTextureView[] = normalTextures.map(
    (nt) => nt?.view ?? flatNormalView,
  );

  // ---- Normal-map pipelines (render to rgba8unorm normal buffer) ----
  const normalBlendTargets: GPUColorTargetState[] = [{
    format: 'rgba8unorm',
    blend: {
      color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
      alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
    },
  }];

  const normalPipelines: GPURenderPipeline[] = manifest.atlases.map((atlas) =>
    device.createRenderPipeline({
      layout: tilePipelineLayout,
      vertex: {
        module: shaderModule,
        entryPoint: 'vs_main',
        constants: { ATLAS_COLS: atlas.cols, ATLAS_ROWS: atlas.rows },
      },
      fragment: {
        module: shaderModule,
        entryPoint: 'fs_normal',
        targets: normalBlendTargets,
      },
      primitive: { topology: 'triangle-list' },
    })
  );

  const normalTextureBindGroups: GPUBindGroup[] = normalMapViews.map((view) =>
    device.createBindGroup({
      layout: textureBindGroupLayout,
      entries: [
        { binding: 0, resource: view },
        { binding: 1, resource: sampler },
      ],
    })
  );

  // ---- Layer Compositor (for baked layers) ----
  let compositor = new LayerCompositor(device, format, canvas.width, canvas.height);

  // ---- Lighting Pipeline (fullscreen post-process) ----
  const LIGHT_FLOATS = 8;
  const MAX_LIGHTS_GPU = 64;

  const lightingShaderModule = device.createShaderModule({ code: lightingShaderSource });

  // Uniform buffer: LightUniforms = 2 × vec4<f32> = 32 bytes
  const lightUniformBuffer = device.createBuffer({
    size: 32,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  // Storage buffer: array<PointLight>, each 8 × f32 = 32 bytes
  const lightStorageBuffer = device.createBuffer({
    size: MAX_LIGHTS_GPU * LIGHT_FLOATS * 4,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });

  const lightingBindGroupLayout = device.createBindGroupLayout({
    entries: [
      { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
      { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
      { binding: 2, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
      { binding: 3, visibility: GPUShaderStage.FRAGMENT, buffer: { type: 'read-only-storage' } },
      { binding: 4, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
    ],
  });

  const lightingPipeline = device.createRenderPipeline({
    layout: device.createPipelineLayout({ bindGroupLayouts: [lightingBindGroupLayout] }),
    vertex: {
      module: lightingShaderModule,
      entryPoint: 'vs_lighting',
    },
    fragment: {
      module: lightingShaderModule,
      entryPoint: 'fs_lighting',
      targets: [{ format }],
    },
    primitive: { topology: 'triangle-list' },
  });

  const lightingSampler = device.createSampler({
    magFilter: 'linear',
    minFilter: 'linear',
  });

  // Scratch texture for scene render (created on demand, resized with canvas)
  let scratchTexture: GPUTexture | null = null;
  let scratchView: GPUTextureView | null = null;
  // Normal buffer for deferred normal-map sampling (same resolution as scratch)
  let normalBuffer: GPUTexture | null = null;
  let normalBufferView: GPUTextureView | null = null;
  let lightingBindGroup: GPUBindGroup | null = null;

  function ensureScratchTexture(w: number, h: number) {
    if (scratchTexture && scratchTexture.width === w && scratchTexture.height === h) return;
    scratchTexture?.destroy();
    normalBuffer?.destroy();
    scratchTexture = device.createTexture({
      size: { width: w, height: h },
      format,
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
    });
    scratchView = scratchTexture.createView();
    // Normal buffer: rgba8unorm (stores tangent-space normals encoded as [0,1])
    normalBuffer = device.createTexture({
      size: { width: w, height: h },
      format: 'rgba8unorm',
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
    });
    normalBufferView = normalBuffer.createView();
    // Recreate bind group with new texture views
    lightingBindGroup = device.createBindGroup({
      layout: lightingBindGroupLayout,
      entries: [
        { binding: 0, resource: scratchView },
        { binding: 1, resource: lightingSampler },
        { binding: 2, resource: { buffer: lightUniformBuffer } },
        { binding: 3, resource: { buffer: lightStorageBuffer } },
        { binding: 4, resource: normalBufferView },
      ],
    });
  }

  // ---- Camera Projection ----
  function updateCamera(width: number, height: number) {
    device.queue.writeBuffer(cameraBuffer, 0, buildProjectionMatrix(width, height, gameWidth, gameHeight));
  }

  updateCamera(canvas.width, canvas.height);

  // ---- Helper: draw a range of instances using the correct atlas pipeline ----
  function drawBatchInstances(
    pass: GPURenderPassEncoder,
    batchStart: number,
    batchEnd: number,
    batchAtlasSplit: number,
  ) {
    // Atlas 0 portion: [batchStart..batchAtlasSplit)
    const atlas0Count = batchAtlasSplit - batchStart;
    if (atlas0Count > 0 && alphaPipelines.length > 0) {
      pass.setPipeline(alphaPipelines[0]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, textureBindGroups[0]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas0Count, 0, batchStart);
    }

    // Atlas 1+ portion: [batchAtlasSplit..batchEnd)
    const atlas1Count = batchEnd - batchAtlasSplit;
    if (atlas1Count > 0 && alphaPipelines.length > 1) {
      pass.setPipeline(alphaPipelines[1]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, textureBindGroups[1]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas1Count, 0, batchAtlasSplit);
    } else if (atlas1Count > 0 && alphaPipelines.length === 1) {
      // Single atlas: draw remaining with same pipeline
      pass.setPipeline(alphaPipelines[0]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, textureBindGroups[0]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas1Count, 0, batchAtlasSplit);
    }
  }

  // ---- Helper: draw a range of instances using normal-map pipelines ----
  function drawNormalBatchInstances(
    pass: GPURenderPassEncoder,
    batchStart: number,
    batchEnd: number,
    batchAtlasSplit: number,
  ) {
    const atlas0Count = batchAtlasSplit - batchStart;
    if (atlas0Count > 0 && normalPipelines.length > 0) {
      pass.setPipeline(normalPipelines[0]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, normalTextureBindGroups[0]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas0Count, 0, batchStart);
    }

    const atlas1Count = batchEnd - batchAtlasSplit;
    if (atlas1Count > 0 && normalPipelines.length > 1) {
      pass.setPipeline(normalPipelines[1]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, normalTextureBindGroups[1]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas1Count, 0, batchAtlasSplit);
    } else if (atlas1Count > 0 && normalPipelines.length === 1) {
      pass.setPipeline(normalPipelines[0]);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, normalTextureBindGroups[0]);
      pass.setBindGroup(2, instanceBindGroup);
      pass.draw(6, atlas1Count, 0, batchAtlasSplit);
    }
  }

  // ---- Draw Function ----
  function draw(
    instanceData: Float32Array,
    instanceCount: number,
    atlasSplit: number,
    effectsData?: Float32Array,
    effectsVertexCount?: number,
    sdfData?: Float32Array,
    sdfInstanceCount?: number,
    vectorData?: Float32Array,
    vectorVertexCount?: number,
    layerBatches?: LayerBatchDescriptor[],
    bakeState?: BakeState,
    lightingState?: LightingState,
  ) {
    const byteLen = instanceCount * INSTANCE_STRIDE;
    device.queue.writeBuffer(instanceBuffer, 0, instanceData.buffer, instanceData.byteOffset, byteLen);

    const hasEffects = effectsData && effectsVertexCount && effectsVertexCount > 0;
    if (hasEffects) {
      const effectsByteLen = effectsVertexCount * EFFECTS_VERTEX_BYTES;
      device.queue.writeBuffer(effectsBuffer, 0, effectsData.buffer, effectsData.byteOffset, effectsByteLen);
    }

    const hasSdf = sdfData && sdfInstanceCount && sdfInstanceCount > 0;
    if (hasSdf) {
      const sdfByteLen = sdfInstanceCount * SDF_INSTANCE_STRIDE;
      device.queue.writeBuffer(sdfStorageBuffer, 0, sdfData.buffer, sdfData.byteOffset, sdfByteLen);
    }

    const hasVectors = vectorData && vectorVertexCount && vectorVertexCount > 0;
    if (hasVectors) {
      const vectorByteLen = vectorVertexCount * VECTOR_VERTEX_BYTES;
      device.queue.writeBuffer(vectorBuffer, 0, vectorData.buffer, vectorData.byteOffset, vectorByteLen);
    }

    // Determine if lighting post-process is needed
    const hasLighting = !!lightingState;

    // Upload light data to GPU when active
    if (hasLighting) {
      const { projWidth, projHeight } = computeProjection(canvas.width, canvas.height, gameWidth, gameHeight);
      // LightUniforms: ambient_and_count (vec4), proj_size (vec4) = 32 bytes
      const uniforms = new Float32Array([
        lightingState.ambient[0], lightingState.ambient[1], lightingState.ambient[2],
        lightingState.lightCount,
        projWidth, projHeight, 0, 0,
      ]);
      device.queue.writeBuffer(lightUniformBuffer, 0, uniforms);

      if (lightingState.lightCount > 0) {
        device.queue.writeBuffer(
          lightStorageBuffer, 0,
          lightingState.lightData.buffer,
          lightingState.lightData.byteOffset,
          lightingState.lightCount * LIGHT_FLOATS * 4,
        );
      }

      // Ensure scratch texture exists at correct size
      ensureScratchTexture(canvas.width, canvas.height);
    }

    const encoder = device.createCommandEncoder();
    const hasBaking = bakeState && bakeState.bakedMask !== 0 && layerBatches && layerBatches.length > 0;

    // ---- Phase 1: Render baked+dirty layers to intermediate textures ----
    if (hasBaking) {
      for (const batch of layerBatches) {
        if (!LayerCompositor.isLayerBaked(bakeState.bakedMask, batch.layerId)) continue;
        if (!compositor.needsRefresh(batch.layerId, bakeState.bakeGen)) continue;

        // Render this layer's instances to an intermediate texture
        const { view: targetView } = compositor.getOrCreateTarget(batch.layerId);
        const layerPass = encoder.beginRenderPass({
          colorAttachments: [{
            view: targetView,
            clearValue: { r: 0, g: 0, b: 0, a: 0 },
            loadOp: 'clear',
            storeOp: 'store',
          }],
        });
        drawBatchInstances(layerPass, batch.start, batch.end, batch.atlasSplit);
        layerPass.end();

        compositor.markClean(batch.layerId, bakeState.bakeGen);
      }
    }

    // ---- Phase 2: Main scene render ----
    // When lighting is active, render to scratch texture; otherwise render directly to screen.
    const screenView = context!.getCurrentTexture().createView();
    const sceneTarget = hasLighting ? scratchView! : screenView;

    const pass = encoder.beginRenderPass({
      colorAttachments: [{
        view: sceneTarget,
        clearValue: { r: 0.02, g: 0.02, b: 0.05, a: 1.0 },
        loadOp: 'clear',
        storeOp: 'store',
      }],
    });

    // Draw sprite instances — layered with baking support
    if (layerBatches && layerBatches.length > 0) {
      for (const batch of layerBatches) {
        if (hasBaking && LayerCompositor.isLayerBaked(bakeState!.bakedMask, batch.layerId)) {
          // Blit cached texture for this layer
          const bindGroup = compositor.getBindGroup(batch.layerId);
          if (bindGroup) {
            pass.setPipeline(compositor.getPipeline());
            pass.setBindGroup(0, bindGroup);
            pass.draw(3); // Fullscreen triangle
          }
        } else {
          // Live layer: render instances directly
          drawBatchInstances(pass, batch.start, batch.end, batch.atlasSplit);
        }
      }
    } else {
      // Legacy path: single atlas split (no baking possible without layer batches)
      drawBatchInstances(pass, 0, instanceCount, atlasSplit);
    }

    // Vector geometry (alpha blend, drawn between sprites and SDF)
    if (hasVectors) {
      pass.setPipeline(vectorPipeline);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setVertexBuffer(0, vectorBuffer);
      pass.draw(vectorVertexCount!);
    }

    // SDF molecules (alpha blend, drawn between vectors and effects)
    if (hasSdf) {
      pass.setPipeline(sdfPipeline);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, sdfBindGroup);
      pass.draw(6, sdfInstanceCount!);
    }

    // Effects (additive blend)
    if (hasEffects) {
      pass.setPipeline(additivePipeline);
      pass.setBindGroup(0, cameraBindGroup);
      pass.setBindGroup(1, textureBindGroups[0] ?? fallbackTextureBindGroup);
      pass.setBindGroup(2, emptyBindGroup);
      pass.setBindGroup(3, colorsBindGroup);
      pass.setVertexBuffer(0, effectsBuffer);
      pass.draw(effectsVertexCount!);
    }

    pass.end();

    // ---- Phase 2b: Normal buffer render (when lighting + normal maps active) ----
    if (hasLighting && hasNormalMaps && normalBufferView) {
      const normalPass = encoder.beginRenderPass({
        colorAttachments: [{
          view: normalBufferView,
          // Clear to flat normal: (0.5, 0.5, 1.0) = tangent-space (0,0,1)
          clearValue: { r: 0.502, g: 0.502, b: 1.0, a: 1.0 },
          loadOp: 'clear',
          storeOp: 'store',
        }],
      });

      // Render sprites using normal atlas textures
      if (layerBatches && layerBatches.length > 0) {
        for (const batch of layerBatches) {
          // Skip baked layers for normal pass (baked layer normals not cached yet)
          drawNormalBatchInstances(normalPass, batch.start, batch.end, batch.atlasSplit);
        }
      } else {
        drawNormalBatchInstances(normalPass, 0, instanceCount, atlasSplit);
      }

      normalPass.end();
    }

    // ---- Phase 3: Lighting post-process (scratch → screen) ----
    if (hasLighting && lightingBindGroup) {
      const lightPass = encoder.beginRenderPass({
        colorAttachments: [{
          view: screenView,
          clearValue: { r: 0, g: 0, b: 0, a: 1.0 },
          loadOp: 'clear',
          storeOp: 'store',
        }],
      });
      lightPass.setPipeline(lightingPipeline);
      lightPass.setBindGroup(0, lightingBindGroup);
      lightPass.draw(3); // Fullscreen triangle
      lightPass.end();
    }

    device.queue.submit([encoder.finish()]);
  }

  function resize(width: number, height: number) {
    canvas.width = width;
    canvas.height = height;
    const configOpts: GPUCanvasConfiguration = { device, format, alphaMode: 'premultiplied' };
    if (tier === 'hdr-edr') {
      configOpts.colorSpace = 'display-p3';
      configOpts.toneMapping = { mode: 'extended' };
    }
    context!.configure(configOpts);
    updateCamera(width, height);
    compositor.resize(width, height);
  }

  return { backend: 'webgpu', tier, draw, resize };
}
