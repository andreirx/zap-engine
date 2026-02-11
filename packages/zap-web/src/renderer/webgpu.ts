// WebGPU renderer — orchestration facade.
// Reads simulation state from SharedArrayBuffer and draws.
// Configures rgba16float + display-p3 + extended tone mapping for HDR/EDR.
// Manifest-driven: accepts N atlases, creates one pipeline per atlas.

import { buildProjectionMatrix, computeProjection } from './camera';
import type { Renderer, LayerBatchDescriptor, BakeState, LightingState } from './types';
import type { AssetManifest } from '../assets/manifest';
import { LayerCompositor } from './compositor';

// Device and resources
import { initDevice, resizeContext, GLOW_MULT } from './webgpu/device';
import {
  loadAtlasTextures,
  loadNormalTextures,
  createSampler,
  createFlatNormalTexture,
  createFallbackTexture,
  createLayouts,
  createBuffers,
  createBindGroups,
  createTextureBindGroups,
  createNormalTextureBindGroups,
  INSTANCE_STRIDE_BYTES,
  EFFECTS_VERTEX_BYTES,
  SDF_INSTANCE_STRIDE_BYTES,
  VECTOR_VERTEX_BYTES,
} from './webgpu/resources';

// Pipelines
import {
  createSpritePipelineLayout,
  createEffectsPipelineLayout,
  createSdfPipelineLayout,
  createVectorPipelineLayout,
} from './webgpu/pipelines/common';
import { createSpriteShaderModule, createSpritePipelines, createNormalPipelines } from './webgpu/pipelines/sprite';
import { createEffectsPipeline } from './webgpu/pipelines/effects';
import { createSdfPipeline, createSdfNormalPipeline } from './webgpu/pipelines/sdf';
import { createVectorPipeline } from './webgpu/pipelines/vector';
import { createLightingPipeline, createLightingBindGroup, LIGHT_FLOATS } from './webgpu/pipelines/lighting';

// Passes
import { encodeBakePass } from './webgpu/passes/bake';
import {
  createDrawBatchFn,
  createDrawNormalBatchFn,
  encodeScenePass,
  encodeNormalPass,
  encodeLightingPass,
  type ScenePassConfig,
  type NormalPassConfig,
} from './webgpu/passes/scene';

// Default capacities (matching GameConfig::default())
const DEFAULT_MAX_INSTANCES = 512;
const DEFAULT_MAX_EFFECTS_VERTICES = 16384;
const DEFAULT_MAX_SDF_INSTANCES = 128;
const DEFAULT_MAX_VECTOR_VERTICES = 16384;
const DEFAULT_MAX_LIGHTS = 64;

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

  // ---- Phase 1: Device initialization ----
  const { device, context, format, tier } = await initDevice(canvas);
  const glowMult = GLOW_MULT[tier as Exclude<typeof tier, 'canvas2d'>];

  // ---- Phase 2: Load textures ----
  const textures = await loadAtlasTextures(device, manifest, atlasBlobs);
  const normalTextures = await loadNormalTextures(device, manifest, normalMapBlobs);
  const hasNormalMaps = normalTextures.some((t) => t !== null);

  const sampler = createSampler(device);
  const flatNormalView = createFlatNormalTexture(device);
  const fallbackTex = createFallbackTexture(device);

  // ---- Phase 3: Create layouts and buffers ----
  const layouts = createLayouts(device);
  const buffers = createBuffers(
    device,
    maxInstances,
    maxEffectsVertices,
    maxSdfInstances,
    maxVectorVertices,
    DEFAULT_MAX_LIGHTS,
  );
  const bindGroups = createBindGroups(device, layouts, buffers);

  // Create texture bind groups
  const textureBindGroups = createTextureBindGroups(device, layouts.textureBindGroupLayout, textures, sampler);
  const normalTextureBindGroups = createNormalTextureBindGroups(
    device,
    layouts.textureBindGroupLayout,
    normalTextures,
    flatNormalView,
    sampler,
  );

  // Fallback texture bind group
  const fallbackTextureBindGroup = device.createBindGroup({
    layout: layouts.textureBindGroupLayout,
    entries: [
      { binding: 0, resource: fallbackTex.createView() },
      { binding: 1, resource: sampler },
    ],
  });

  // ---- Phase 4: Create pipelines ----
  const spritePipelineLayout = createSpritePipelineLayout(device, layouts);
  const effectsPipelineLayout = createEffectsPipelineLayout(device, layouts);
  const sdfPipelineLayout = createSdfPipelineLayout(device, layouts);
  const vectorPipelineLayout = createVectorPipelineLayout(device, layouts);

  const spriteShaderModule = createSpriteShaderModule(device);
  const alphaPipelines = createSpritePipelines(
    { device, layout: spritePipelineLayout, format, manifest },
    spriteShaderModule,
  );
  const normalPipelines = createNormalPipelines(
    { device, layout: spritePipelineLayout, format, manifest },
    spriteShaderModule,
  );

  const additivePipeline = createEffectsPipeline({
    device,
    layout: effectsPipelineLayout,
    format,
    glowMult: glowMult.effects,
  });

  const sdfPipeline = createSdfPipeline({
    device,
    layout: sdfPipelineLayout,
    format,
    emissiveMult: glowMult.sdf,
  });

  // SDF normal pipeline: writes flat normals so sprite normals don't bleed onto SDF shapes
  const sdfNormalPipeline = createSdfNormalPipeline({
    device,
    layout: sdfPipelineLayout,
    format: 'rgba8unorm',  // Normal buffer format
  });

  const vectorPipeline = createVectorPipeline({
    device,
    layout: vectorPipelineLayout,
    format,
    glowMult: glowMult.vector,
  });

  const lighting = createLightingPipeline({ device, format });

  // ---- Phase 5: Create compositor and scratch textures ----
  let compositor = new LayerCompositor(device, format, canvas.width, canvas.height);

  let scratchTexture: GPUTexture | null = null;
  let scratchView: GPUTextureView | null = null;
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
    normalBuffer = device.createTexture({
      size: { width: w, height: h },
      format: 'rgba8unorm',
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
    });
    normalBufferView = normalBuffer.createView();
    lightingBindGroup = createLightingBindGroup(
      device,
      lighting.bindGroupLayout,
      scratchView,
      normalBufferView,
      lighting.sampler,
      buffers,
    );
  }

  // ---- Phase 6: Camera setup ----
  function updateCamera(width: number, height: number) {
    device.queue.writeBuffer(buffers.cameraBuffer, 0, buildProjectionMatrix(width, height, gameWidth, gameHeight));
  }
  updateCamera(canvas.width, canvas.height);

  // ---- Create scene pass config ----
  const sceneConfig: ScenePassConfig = {
    alphaPipelines,
    normalPipelines,
    vectorPipeline,
    sdfPipeline,
    additivePipeline,
    cameraBindGroup: bindGroups.cameraBindGroup,
    textureBindGroups,
    normalTextureBindGroups,
    instanceBindGroup: bindGroups.instanceBindGroup,
    sdfBindGroup: bindGroups.sdfBindGroup,
    colorsBindGroup: bindGroups.colorsBindGroup,
    emptyBindGroup: bindGroups.emptyBindGroup,
    fallbackTextureBindGroup,
    effectsBuffer: buffers.effectsBuffer,
    vectorBuffer: buffers.vectorBuffer,
    compositor,
  };

  const drawBatchInstances = createDrawBatchFn(sceneConfig);
  const drawNormalBatchInstances = createDrawNormalBatchFn(sceneConfig);

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
    // Upload instance data
    const byteLen = instanceCount * INSTANCE_STRIDE_BYTES;
    device.queue.writeBuffer(buffers.instanceBuffer, 0, instanceData.buffer, instanceData.byteOffset, byteLen);

    // Upload effects data
    const hasEffects = effectsData && effectsVertexCount && effectsVertexCount > 0;
    if (hasEffects) {
      const effectsByteLen = effectsVertexCount * EFFECTS_VERTEX_BYTES;
      device.queue.writeBuffer(buffers.effectsBuffer, 0, effectsData.buffer, effectsData.byteOffset, effectsByteLen);
    }

    // Upload SDF data
    const hasSdf = sdfData && sdfInstanceCount && sdfInstanceCount > 0;
    if (hasSdf) {
      const sdfByteLen = sdfInstanceCount * SDF_INSTANCE_STRIDE_BYTES;
      device.queue.writeBuffer(buffers.sdfStorageBuffer, 0, sdfData.buffer, sdfData.byteOffset, sdfByteLen);
    }

    // Upload vector data
    const hasVectors = vectorData && vectorVertexCount && vectorVertexCount > 0;
    if (hasVectors) {
      const vectorByteLen = vectorVertexCount * VECTOR_VERTEX_BYTES;
      device.queue.writeBuffer(buffers.vectorBuffer, 0, vectorData.buffer, vectorData.byteOffset, vectorByteLen);
    }

    // Determine if lighting post-process is needed
    const hasLighting = !!lightingState;

    // Upload light data when active
    if (hasLighting) {
      const { projWidth, projHeight } = computeProjection(canvas.width, canvas.height, gameWidth, gameHeight);
      const uniforms = new Float32Array([
        lightingState.ambient[0], lightingState.ambient[1], lightingState.ambient[2],
        lightingState.lightCount,
        projWidth, projHeight, 0, 0,
      ]);
      device.queue.writeBuffer(buffers.lightUniformBuffer, 0, uniforms);

      if (lightingState.lightCount > 0) {
        device.queue.writeBuffer(
          buffers.lightStorageBuffer, 0,
          lightingState.lightData.buffer,
          lightingState.lightData.byteOffset,
          lightingState.lightCount * LIGHT_FLOATS * 4,
        );
      }

      ensureScratchTexture(canvas.width, canvas.height);
    }

    const encoder = device.createCommandEncoder();
    const hasBaking = bakeState && bakeState.bakedMask !== 0 && layerBatches && layerBatches.length > 0;

    // ---- Phase 1: Bake pass (render dirty baked layers to textures) ----
    if (hasBaking) {
      encodeBakePass(encoder, compositor, layerBatches, bakeState, drawBatchInstances);
    }

    // ---- Phase 2: Main scene render ----
    const screenView = context.getCurrentTexture().createView();
    const sceneTarget = hasLighting ? scratchView! : screenView;

    const pass = encoder.beginRenderPass({
      colorAttachments: [{
        view: sceneTarget,
        clearValue: { r: 0.02, g: 0.02, b: 0.05, a: 1.0 },
        loadOp: 'clear',
        storeOp: 'store',
      }],
    });

    encodeScenePass(
      pass,
      sceneConfig,
      drawBatchInstances,
      instanceCount,
      atlasSplit,
      layerBatches,
      bakeState,
      hasEffects ? effectsVertexCount! : 0,
      hasSdf ? sdfInstanceCount! : 0,
      hasVectors ? vectorVertexCount! : 0,
    );

    pass.end();

    // ---- Phase 2b: Normal buffer render (when lighting + normal maps active) ----
    if (hasLighting && hasNormalMaps && normalBufferView) {
      const normalPass = encoder.beginRenderPass({
        colorAttachments: [{
          view: normalBufferView,
          clearValue: { r: 0.502, g: 0.502, b: 1.0, a: 1.0 },
          loadOp: 'clear',
          storeOp: 'store',
        }],
      });

      const normalPassConfig: NormalPassConfig = {
        sdfNormalPipeline,
        cameraBindGroup: bindGroups.cameraBindGroup,
        sdfBindGroup: bindGroups.sdfBindGroup,
      };
      encodeNormalPass(normalPass, drawNormalBatchInstances, instanceCount, atlasSplit, layerBatches, hasSdf ? sdfInstanceCount! : 0, normalPassConfig);
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

      encodeLightingPass(lightPass, lighting.pipeline, lightingBindGroup);
      lightPass.end();
    }

    device.queue.submit([encoder.finish()]);
  }

  // ---- Resize Function ----
  function resize(width: number, height: number) {
    resizeContext(canvas, context, device, format, tier, width, height);
    updateCamera(width, height);
    compositor.resize(width, height);
  }

  return { backend: 'webgpu', tier, draw, resize };
}
