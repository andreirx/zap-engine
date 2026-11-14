// When the WebGPU tier writes the normal buffer (R-FLAT-NORMAL, ADR-042).
//
// The lighting pass (lighting.wgsl) reads a normal for every pixel from the
// normal buffer. The normal-buffer pass is the only writer of that buffer: it
// clears it to the flat normal (0.502, 0.502, 1.0), which decodes to (0, 0, 1),
// then draws the sprites' normals and the SDF shapes' flat normals over it.
// These two functions are the one statement of when that pass runs and what it
// draws; both normal-buffer passes in webgpu.ts (the visibility path and the
// standard path) call them.
//
// No value import: Node's type stripping resolves only extension-qualified
// imports (ADR-040), and the test imports this file directly.

/**
 * The normal-buffer pass runs whenever lighting is on and the buffer exists.
 * It does not depend on normal maps: without them the pass still clears the
 * buffer to the flat normal, so that every pixel the lighting pass reads holds
 * a normal. Before ADR-042 the pass also required a normal map; with lighting
 * on and none, the buffer kept its initial zero, which decodes to (−1, −1, −1).
 */
export function normalPassRuns(hasLighting: boolean, hasNormalBuffer: boolean): boolean {
  return hasLighting && hasNormalBuffer;
}

/**
 * The normal-buffer pass draws the sprites' normals only when at least one
 * atlas has a normal map. Without one, the sprites keep the cleared flat
 * normal. The SDF shapes' flat normals are drawn in either case.
 */
export function drawsSpriteNormals(hasNormalMaps: boolean): boolean {
  return hasNormalMaps;
}
