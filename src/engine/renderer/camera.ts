// Shared camera/projection math — used by both WebGPU and Canvas 2D renderers.
// Parameterized: accepts gameWidth/gameHeight instead of using hardcoded constants.

export interface Projection {
  projWidth: number;
  projHeight: number;
  scaleX: number;
  scaleY: number;
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

/** Build column-major orthographic projection matrix for WebGPU. */
export function buildProjectionMatrix(
  canvasW: number,
  canvasH: number,
  gameW: number,
  gameH: number,
): Float32Array {
  const { projWidth, projHeight } = computeProjection(canvasW, canvasH, gameW, gameH);
  const l = 0, r = projWidth, b = projHeight, t = 0;
  return new Float32Array([
    2 / (r - l), 0, 0, 0,
    0, 2 / (t - b), 0, 0,
    0, 0, 1, 0,
    -(r + l) / (r - l), -(t + b) / (t - b), 0, 1,
  ]);
}
