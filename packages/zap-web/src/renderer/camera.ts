// Shared camera/projection math — used by both WebGPU and Canvas 2D renderers.
// Parameterized: accepts gameWidth/gameHeight instead of using hardcoded constants.
//
// No value import here: camera.test.ts loads this file with Node's type
// stripping, which resolves only extension-qualified value imports (ADR-040).

export interface Projection {
  projWidth: number;
  projHeight: number;
  scaleX: number;
  scaleY: number;
}

/**
 * A 2D camera: the world point shown at the canvas's centre, and the zoom.
 * At zoom 1 the canvas shows the area of `computeProjection` (the game area
 * extended on its long side to the canvas's aspect); at zoom z the shown width
 * and height are that area's divided by z. zap-web clamps nothing (ADR-040).
 */
export interface Camera {
  x: number;
  y: number;
  zoom: number;
}

/** A world-space rectangle: the area the canvas shows. */
export interface CameraView {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Compute aspect-preserving projection dimensions and scale factors. */
export function computeProjection(
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
): Projection {
  const aspect = canvasW / canvasH;
  const gameAspect = gameW / gameH;
  let projWidth = gameW;
  let projHeight = gameH;
  if (aspect > gameAspect) {
    projWidth = gameH * aspect;
  } else {
    projHeight = gameW / aspect;
  }
  return {
    projWidth,
    projHeight,
    scaleX: canvasW / projWidth,
    scaleY: canvasH / projHeight,
  };
}

/**
 * Throw a RangeError naming the field when a camera is not usable:
 * a non-finite x, y or zoom, or a zoom not above 0. The only check zap-web
 * makes on a camera (ADR-040); Renderer.setCamera calls it.
 */
export function checkCamera(camera: Camera): void {
  if (!Number.isFinite(camera.x)) throw new RangeError(`Camera: x must be finite, got ${camera.x}`);
  if (!Number.isFinite(camera.y)) throw new RangeError(`Camera: y must be finite, got ${camera.y}`);
  if (!Number.isFinite(camera.zoom) || camera.zoom <= 0) {
    throw new RangeError(`Camera: zoom must be finite and above 0, got ${camera.zoom}`);
  }
}

/**
 * True when `next` differs from `previous`: one is null and the other is not,
 * or a field differs. Renderer.setCamera invalidates baked-layer caches on true.
 */
export function cameraChanged(previous: Camera | null, next: Camera | null): boolean {
  if (previous === null || next === null) return previous !== next;
  return previous.x !== next.x || previous.y !== next.y || previous.zoom !== next.zoom;
}

/**
 * The world rectangle the canvas shows. With no camera: today's area,
 * `{ left: 0, top: 0, width: projWidth, height: projHeight }`. With a camera:
 * that area's size divided by the zoom, centred on (camera.x, camera.y).
 */
export function cameraView(
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
  camera?: Camera | null,
): CameraView {
  const { projWidth, projHeight } = computeProjection(canvasW, canvasH, gameW, gameH);
  if (!camera) return { left: 0, top: 0, width: projWidth, height: projHeight };
  const width = projWidth / camera.zoom;
  const height = projHeight / camera.zoom;
  return { left: camera.x - width / 2, top: camera.y - height / 2, width, height };
}

/**
 * The world rectangle a game asks the renderer to show (ADR-043): centre
 * (x, y) in the renderer's world (origin top-left, Y down), size width × height.
 * The frame header carries it (HEADER_CAMERA_*).
 */
export interface CameraRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * The Camera that shows `rect` whole and centred (ADR-043): centre (rect.x,
 * rect.y), zoom min(projWidth / rect.width, projHeight / rect.height). When the
 * canvas's aspect differs from the rectangle's, the canvas shows more of the
 * world along its longer side. No check here. setCamera's checkCamera checks
 * the converted camera only (centre finite, zoom finite and above 0), not the
 * rectangle: one dimension 0 drops out of the min and is accepted (the zoom
 * comes from the other); both 0, a negative or NaN dimension, or an infinite
 * one give a zoom it rejects (camera.test.ts ZE3 2f).
 */
export function cameraFromRect(
  rect: CameraRect,
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
): Camera {
  const { projWidth, projHeight } = computeProjection(canvasW, canvasH, gameW, gameH);
  return { x: rect.x, y: rect.y, zoom: Math.min(projWidth / rect.width, projHeight / rect.height) };
}

/**
 * Build column-major orthographic projection matrix for WebGPU.
 * Maps cameraView's rectangle to clip space: left → −1, right → +1,
 * top → +1, bottom → −1. With no camera, the matrix of today.
 */
export function buildProjectionMatrix(
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
  camera?: Camera | null,
): Float32Array {
  const { left, top, width, height } = cameraView(canvasW, canvasH, gameW, gameH, camera);
  // With no camera, left = top = 0, so l, r, t, b are today's exact values.
  const l = left, r = left + width, b = top + height, t = top;
  return new Float32Array([
    2 / (r - l), 0, 0, 0,
    0, 2 / (t - b), 0, 0,
    0, 0, 1, 0,
    -(r + l) / (r - l), -(t + b) / (t - b), 0, 1,
  ]);
}

/**
 * Canvas pixel → world point. Pixels are the canvas's own (canvas.width units).
 * With no camera, the worker's mapping of today (engine.worker.ts screenToWorld)
 * for a canvas of positive size.
 */
export function screenToWorld(
  px: number,
  py: number,
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
  camera?: Camera | null,
): { x: number; y: number } {
  const { left, top, width, height } = cameraView(canvasW, canvasH, gameW, gameH, camera);
  return {
    x: left + px * width / canvasW,
    y: top + py * height / canvasH,
  };
}

/** World point → canvas pixel; the inverse of screenToWorld. */
export function worldToScreen(
  wx: number,
  wy: number,
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
  camera?: Camera | null,
): { x: number; y: number } {
  const { left, top, width, height } = cameraView(canvasW, canvasH, gameW, gameH, camera);
  return {
    x: (wx - left) * canvasW / width,
    y: (wy - top) * canvasH / height,
  };
}

/**
 * The camera after multiplying its zoom by `factor`, with the world point under
 * canvas pixel (px, py) unchanged. No clamp: a result with zoom ≤ 0 (factor ≤ 0)
 * is rejected by checkCamera when given to setCamera.
 */
export function zoomAbout(
  camera: Camera,
  px: number,
  py: number,
  factor: number,
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
): Camera {
  const anchor = screenToWorld(px, py, canvasW, canvasH, gameW, gameH, camera);
  const zoom = camera.zoom * factor;
  const { projWidth, projHeight } = computeProjection(canvasW, canvasH, gameW, gameH);
  const width = projWidth / zoom;
  const height = projHeight / zoom;
  const left = anchor.x - px * width / canvasW;
  const top = anchor.y - py * height / canvasH;
  return { x: left + width / 2, y: top + height / 2, zoom };
}

/**
 * The WebGPU lighting uniform (lighting.wgsl LightUniforms):
 * [ambient r, g, b, lightCount, view width, view height, view left, view top].
 * With no camera: today's [ambient…, count, projWidth, projHeight, 0, 0].
 */
export function lightUniform(
  ambient: readonly [number, number, number],
  lightCount: number,
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
  camera?: Camera | null,
): Float32Array {
  const { left, top, width, height } = cameraView(canvasW, canvasH, gameW, gameH, camera);
  return new Float32Array([
    ambient[0], ambient[1], ambient[2],
    lightCount,
    width, height, left, top,
  ]);
}

/**
 * The visibility-mask coordinate of a canvas pixel (rule R-VISIBILITY-MASK,
 * ADR-040), as { u, v } in 0…1, or null when the pixel is not darkened.
 * - No camera: WebGPU's meaning of today, the screen uv over the whole canvas
 *   (Canvas2D keeps its own meaning of today, the game rectangle; ADR-040).
 * - Camera: the mask's cols × rows cover the world rectangle 0…gameW × 0…gameH;
 *   a pixel whose world point lies outside it gives null.
 * visibility.wgsl transcribes this function.
 */
export function maskUv(
  canvasX: number,
  canvasY: number,
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
  camera?: Camera | null,
): { u: number; v: number } | null {
  if (!camera) return { u: canvasX / canvasW, v: canvasY / canvasH };
  const world = screenToWorld(canvasX, canvasY, canvasW, canvasH, gameW, gameH, camera);
  const u = world.x / gameW;
  const v = world.y / gameH;
  if (u < 0 || u > 1 || v < 0 || v > 1) return null;
  return { u, v };
}

/**
 * The WebGPU visibility uniform (visibility.wgsl VisibilityUniforms):
 * [view left, top, width, height, gameW, gameH, 0, 0]. With no camera the view
 * is written as all 0: width = 0 is the shader's "no camera" flag.
 */
export function visibilityUniform(
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
  camera?: Camera | null,
): Float32Array {
  if (!camera) return new Float32Array([0, 0, 0, 0, gameW, gameH, 0, 0]);
  const { left, top, width, height } = cameraView(canvasW, canvasH, gameW, gameH, camera);
  return new Float32Array([left, top, width, height, gameW, gameH, 0, 0]);
}
