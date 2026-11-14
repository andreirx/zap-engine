/// <reference types="node" />
// Tests of the camera math (HODA-ZE2, ADR-040). Run with Node's own test runner:
//   node --test packages/zap-web/src/renderer/camera.test.ts
// The import of ./camera.ts carries its extension: Node's type stripping
// resolves only extension-qualified imports. Every expected value is worked by
// hand in the comment beside it.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  buildProjectionMatrix,
  cameraChanged,
  cameraFromRect,
  cameraView,
  checkCamera,
  computeProjection,
  lightUniform,
  maskUv,
  screenToWorld,
  visibilityUniform,
  worldToScreen,
  zoomAbout,
  type Camera,
} from './camera.ts';

const GAME_W = 1024;
const GAME_H = 640;
// Game aspect 1024 / 640 = 1.6.
// Camera of metric 1c: canvas 1000 × 500 (aspect 2 > 1.6, so the projection is
// 640 × 2 = 1280 wide, 640 high); zoom 2 shows 640 × 320 centred on (512, 320):
// left 512 − 320 = 192, top 320 − 160 = 160.
const CAM_C: Camera = { x: 512, y: 320, zoom: 2 };

function near(actual: number, expected: number, tol: number, what: string) {
  assert.ok(Math.abs(actual - expected) <= tol, `${what}: ${actual} vs ${expected}`);
}

// Clip coordinates of a world point under a column-major 4 × 4 matrix (z = 0, w = 1).
function clip(m: Float32Array, x: number, y: number): [number, number] {
  return [m[0] * x + m[4] * y + m[12], m[1] * x + m[5] * y + m[13]];
}

test('1a: no camera gives the projection matrix of today for three canvases', () => {
  // 1000 × 500: aspect 2 > 1.6 → proj 1280 × 640 (l 0, r 1280, t 0, b 640).
  //   m0 = 2/1280 = 0.0015625; m5 = 2/(0 − 640) = −0.003125;
  //   m12 = −1280/1280 = −1; m13 = −640/(0 − 640) = 1.
  assert.deepEqual(buildProjectionMatrix(1000, 500, GAME_W, GAME_H), new Float32Array([
    0.0015625, 0, 0, 0,
    0, -0.003125, 0, 0,
    0, 0, 1, 0,
    -1, 1, 0, 1,
  ]));
  // 500 × 1000: aspect 0.5 < 1.6 → proj 1024 × 2048.
  //   m0 = 2/1024 = 0.001953125; m5 = 2/(−2048) = −0.0009765625; m12 = −1; m13 = 1.
  assert.deepEqual(buildProjectionMatrix(500, 1000, GAME_W, GAME_H), new Float32Array([
    0.001953125, 0, 0, 0,
    0, -0.0009765625, 0, 0,
    0, 0, 1, 0,
    -1, 1, 0, 1,
  ]));
  // 1024 × 640: aspect = game aspect → proj 1024 × 640.
  //   m0 = 2/1024 = 0.001953125; m5 = 2/(−640) = −0.003125; m12 = −1; m13 = 1.
  assert.deepEqual(buildProjectionMatrix(1024, 640, GAME_W, GAME_H), new Float32Array([
    0.001953125, 0, 0, 0,
    0, -0.003125, 0, 0,
    0, 0, 1, 0,
    -1, 1, 0, 1,
  ]));
  // null is the same as no camera.
  assert.deepEqual(
    buildProjectionMatrix(1000, 500, GAME_W, GAME_H, null),
    buildProjectionMatrix(1000, 500, GAME_W, GAME_H),
  );
});

test('1b: a camera at today\'s centre at zoom 1 gives the matrix of 1a', () => {
  for (const [cw, ch] of [[1000, 500], [500, 1000], [1024, 640]]) {
    const { projWidth, projHeight } = computeProjection(cw, ch, GAME_W, GAME_H);
    const cam = { x: projWidth / 2, y: projHeight / 2, zoom: 1 };
    const legacy = buildProjectionMatrix(cw, ch, GAME_W, GAME_H);
    const withCam = buildProjectionMatrix(cw, ch, GAME_W, GAME_H, cam);
    for (let i = 0; i < 16; i++) near(withCam[i], legacy[i], 1e-6, `${cw}×${ch} entry ${i}`);
  }
});

test('1c: cameraView of the zoom-2 camera', () => {
  // proj 1280 × 640 / 2 = 640 × 320; left 512 − 320 = 192; top 320 − 160 = 160.
  assert.deepEqual(cameraView(1000, 500, GAME_W, GAME_H, CAM_C), { left: 192, top: 160, width: 640, height: 320 });
  // No camera: today's area, origin 0.
  assert.deepEqual(cameraView(1000, 500, GAME_W, GAME_H, null), { left: 0, top: 0, width: 1280, height: 640 });
});

test('1d: the matrix of 1c maps the view corners to clip corners', () => {
  const m = buildProjectionMatrix(1000, 500, GAME_W, GAME_H, CAM_C);
  // Top-left (192, 160) → (−1, +1); bottom-right (192 + 640, 160 + 320) = (832, 480) → (+1, −1).
  const [x0, y0] = clip(m, 192, 160);
  near(x0, -1, 1e-6, 'top-left x');
  near(y0, 1, 1e-6, 'top-left y');
  const [x1, y1] = clip(m, 832, 480);
  near(x1, 1, 1e-6, 'bottom-right x');
  near(y1, -1, 1e-6, 'bottom-right y');
});

test('1e: screenToWorld inverts worldToScreen; no camera is the worker formula', () => {
  const points: [number, number][] = [[0, 0], [512, 320], [-100, 900], [1023.5, 0.25], [3000, -2000]];
  const cameras: Camera[] = [CAM_C, { x: -50, y: 75, zoom: 0.25 }, { x: 900, y: 10, zoom: 3.7 }];
  for (const cam of cameras) {
    for (const [wx, wy] of points) {
      const s = worldToScreen(wx, wy, 1000, 500, GAME_W, GAME_H, cam);
      const w = screenToWorld(s.x, s.y, 1000, 500, GAME_W, GAME_H, cam);
      near(w.x, wx, 1e-9, `x of (${wx}, ${wy}) at zoom ${cam.zoom}`);
      near(w.y, wy, 1e-9, `y of (${wx}, ${wy}) at zoom ${cam.zoom}`);
    }
  }
  // engine.worker.ts:160-164 (canvas of positive size):
  //   x = cssX * projWidth / canvasWidth, y = cssY * projHeight / canvasHeight.
  for (const [cw, ch] of [[1000, 500], [500, 1000], [1024, 640]]) {
    const { projWidth, projHeight } = computeProjection(cw, ch, GAME_W, GAME_H);
    for (const [px, py] of [[0, 0], [400, 250], [137, 333], [999, 499], [-20, 1200]]) {
      const w = screenToWorld(px, py, cw, ch, GAME_W, GAME_H);
      assert.equal(w.x, px * projWidth / cw);
      assert.equal(w.y, py * projHeight / ch);
    }
  }
});

test('1f: zoomAbout keeps the world point under the pointer', () => {
  // Before: world under (250, 125) = (192 + 250·640/1000, 160 + 125·320/500) = (352, 240).
  // After (zoom 4, view 320 × 160): left = 352 − 250·320/1000 = 272, top = 240 − 125·160/500 = 200,
  // so the camera is (272 + 160, 200 + 80) = (432, 280).
  const before = screenToWorld(250, 125, 1000, 500, GAME_W, GAME_H, CAM_C);
  const next = zoomAbout(CAM_C, 250, 125, 2, 1000, 500, GAME_W, GAME_H);
  const after = screenToWorld(250, 125, 1000, 500, GAME_W, GAME_H, next);
  near(after.x, before.x, 1e-9, 'anchor x');
  near(after.y, before.y, 1e-9, 'anchor y');
  near(before.x, 352, 1e-9, 'anchor x by hand');
  near(before.y, 240, 1e-9, 'anchor y by hand');
  assert.equal(next.zoom, 4);
  near(next.x, 432, 1e-9, 'camera x by hand');
  near(next.y, 280, 1e-9, 'camera y by hand');
});

test('1g: the camera guard throws a RangeError naming the field', () => {
  assert.throws(() => checkCamera({ x: 0, y: 0, zoom: 0 }), (e: unknown) => e instanceof RangeError && /zoom/.test(e.message));
  assert.throws(() => checkCamera({ x: 0, y: 0, zoom: -1 }), (e: unknown) => e instanceof RangeError && /zoom/.test(e.message));
  assert.throws(() => checkCamera({ x: 0, y: 0, zoom: NaN }), (e: unknown) => e instanceof RangeError && /zoom/.test(e.message));
  assert.throws(() => checkCamera({ x: Infinity, y: 0, zoom: 1 }), (e: unknown) => e instanceof RangeError && /\bx\b/.test(e.message));
  assert.throws(() => checkCamera({ x: 0, y: -Infinity, zoom: 1 }), (e: unknown) => e instanceof RangeError && /\by\b/.test(e.message));
  assert.doesNotThrow(() => checkCamera(CAM_C));
});

test('1h: the lighting uniform carries the view size and origin', () => {
  // No camera: today's eight numbers [ambient…, count, projWidth, projHeight, 0, 0] (webgpu.ts of d8422c9).
  assert.deepEqual(
    lightUniform([0.2, 0.3, 0.4], 3, 1000, 500, GAME_W, GAME_H),
    new Float32Array([0.2, 0.3, 0.4, 3, 1280, 640, 0, 0]),
  );
  // Camera of 1c: width 640, height 320, left 192, top 160.
  assert.deepEqual(
    lightUniform([0.2, 0.3, 0.4], 3, 1000, 500, GAME_W, GAME_H, CAM_C),
    new Float32Array([0.2, 0.3, 0.4, 3, 640, 320, 192, 160]),
  );
});

test('1i: R-VISIBILITY-MASK, the mask mapping with and without a camera', () => {
  // Canvas 1000 × 500, game 1024 × 640, mask 4 × 2.
  const cols = 4, rows = 2;
  // No camera, WebGPU meaning: screen uv. Pixel (400, 250) → (400/1000, 250/500) = (0.4, 0.5).
  assert.deepEqual(maskUv(400, 250, 1000, 500, GAME_W, GAME_H), { u: 0.4, v: 0.5 });
  // No camera, Canvas2D meaning (canvas2d.ts:556-566 of d8422c9): cell (vx, vy) is drawn at canvas
  // (vx·cellW, vy·cellH) with cellW = (gameW / cols)·scaleX = 256·0.78125 = 200 px and
  // cellH = (gameH / rows)·scaleY = 320·0.78125 = 250 px. World (512, 320) is canvas
  // (512·0.78125, 320·0.78125) = (400, 250), so mask cell (400/200, 250/250) = (2, 1).
  const { scaleX, scaleY } = computeProjection(1000, 500, GAME_W, GAME_H);
  const cellWpx = (GAME_W / cols) * scaleX;
  const cellHpx = (GAME_H / rows) * scaleY;
  const p = worldToScreen(512, 320, 1000, 500, GAME_W, GAME_H);
  assert.deepEqual([Math.floor(p.x / cellWpx), Math.floor(p.y / cellHpx)], [2, 1]);
  // The tiers' present difference (ADR-040): WebGPU puts the same canvas pixel at u 0.4, cell 1.
  assert.equal(Math.floor(0.4 * cols), 1);

  // Camera of 1c: pixel (500, 250) is world (192 + 500·0.64, 160 + 250·0.64) = (512, 320),
  // uv (512/1024, 320/640) = (0.5, 0.5): the same cell (2, 1) as Canvas2D's meaning.
  assert.deepEqual(maskUv(500, 250, 1000, 500, GAME_W, GAME_H, CAM_C), { u: 0.5, v: 0.5 });
  // Pixel (0, 0) is world (192, 160), uv (192/1024, 160/640) = (0.1875, 0.25).
  assert.deepEqual(maskUv(0, 0, 1000, 500, GAME_W, GAME_H, CAM_C), { u: 0.1875, v: 0.25 });
  // Camera (−200, 320) zoom 2: left −200 − 320 = −520, so pixel (0, 250) is world (−520, 320):
  // outside the game rectangle → null (not darkened).
  assert.equal(maskUv(0, 250, 1000, 500, GAME_W, GAME_H, { x: -200, y: 320, zoom: 2 }), null);

  // The uniform visibility.wgsl reads: width 0 flags "no camera".
  assert.deepEqual(visibilityUniform(1000, 500, GAME_W, GAME_H), new Float32Array([0, 0, 0, 0, 1024, 640, 0, 0]));
  assert.deepEqual(
    visibilityUniform(1000, 500, GAME_W, GAME_H, CAM_C),
    new Float32Array([192, 160, 640, 320, 1024, 640, 0, 0]),
  );
});

test('1j: cameraChanged', () => {
  assert.equal(cameraChanged(null, null), false);
  assert.equal(cameraChanged({ ...CAM_C }, { ...CAM_C }), false);
  assert.equal(cameraChanged(CAM_C, { ...CAM_C, x: 513 }), true);
  assert.equal(cameraChanged(CAM_C, { ...CAM_C, y: 321 }), true);
  assert.equal(cameraChanged(CAM_C, { ...CAM_C, zoom: 3 }), true);
  assert.equal(cameraChanged(null, CAM_C), true);
  assert.equal(cameraChanged(CAM_C, null), true);
});

// --- HODA-ZE3 (ADR-043): the game's camera rectangle -------------------------

test('ZE3 2a: cameraFromRect fits the rectangle inside canvases of two aspects', () => {
  const rect = { x: 480, y: 270, width: 960, height: 540 };
  // 1920 × 1080 game (aspect 1.778) on 1000 × 500 (aspect 2 > 1.778): projection
  // 1080·2 = 2160 wide, 1080 high; zoom min(2160 / 960, 1080 / 540) = min(2.25, 2) = 2.
  assert.deepEqual(cameraFromRect(rect, 1000, 500, 1920, 1080), { x: 480, y: 270, zoom: 2 });
  // The view is 2160 / 2 × 1080 / 2 = 1080 × 540: the rectangle's height exactly, more width.
  assert.deepEqual(
    cameraView(1000, 500, 1920, 1080, cameraFromRect(rect, 1000, 500, 1920, 1080)),
    { left: 480 - 540, top: 0, width: 1080, height: 540 },
  );
  // On 1000 × 1000 (aspect 1 < 1.778): projection 1920 wide, 1920 / 1 = 1920 high;
  // zoom min(1920 / 960, 1920 / 540) = min(2, 3.556) = 2; view 960 × 960, more height.
  assert.deepEqual(cameraFromRect(rect, 1000, 1000, 1920, 1080), { x: 480, y: 270, zoom: 2 });
  assert.deepEqual(
    cameraView(1000, 1000, 1920, 1080, cameraFromRect(rect, 1000, 1000, 1920, 1080)),
    { left: 0, top: 270 - 480, width: 960, height: 960 },
  );
});

test('ZE3 2b: the worker mapping, with no camera and with the camera of the last header', () => {
  // What engine.worker.ts does: screenToWorld(css point, canvas CSS size, world size,
  // rect ? cameraFromRect(rect, canvas CSS size, world size) : null).
  // The probe's numbers: canvas 1200 × 800 CSS pixels, world 800 × 600 (basic-demo).
  const [cw, ch, gw, gh] = [1200, 800, 800, 600];
  const { projWidth, projHeight } = computeProjection(cw, ch, gw, gh);
  // Aspect 1.5 > 1.333: projection 600·1.5 = 900 × 600.
  assert.deepEqual([projWidth, projHeight], [900, 600]);
  // No camera: today's formula (engine.worker.ts before HODA-ZE3), x · 900 / 1200, y · 600 / 800.
  for (const [px, py] of [[600, 400], [0, 0], [137, 733], [1199, 1]]) {
    assert.deepEqual(screenToWorld(px, py, cw, ch, gw, gh, null), { x: px * projWidth / cw, y: py * projHeight / ch });
  }
  assert.deepEqual(screenToWorld(600, 400, cw, ch, gw, gh, null), { x: 450, y: 300 });
  assert.deepEqual(screenToWorld(0, 0, cw, ch, gw, gh, null), { x: 0, y: 0 });
  // Camera rectangle {400, 300, 400, 300}: zoom min(900 / 400, 600 / 300) = min(2.25, 2) = 2;
  // view 450 × 300, left 400 − 225 = 175, top 300 − 150 = 150.
  const cam = cameraFromRect({ x: 400, y: 300, width: 400, height: 300 }, cw, ch, gw, gh);
  assert.deepEqual(cam, { x: 400, y: 300, zoom: 2 });
  // Centre (600, 400): (175 + 600·450 / 1200, 150 + 400·300 / 800) = (400, 300).
  assert.deepEqual(screenToWorld(600, 400, cw, ch, gw, gh, cam), { x: 400, y: 300 });
  assert.deepEqual(screenToWorld(0, 0, cw, ch, gw, gh, cam), { x: 175, y: 150 });
});

test('ZE3 2f: what the guard sees of a degenerate rectangle is the converted centre and zoom', () => {
  // The probe's canvas and world: projection 900 × 600. setCamera checks only the converted
  // camera (checkCamera: centre finite, zoom finite and above 0), not the rectangle's dimensions.
  const [cw, ch, gw, gh] = [1200, 800, 800, 600];
  const fit = (width: number, height: number, x = 400, y = 300) =>
    cameraFromRect({ x, y, width, height }, cw, ch, gw, gh);
  // One dimension 0 drops out of the min (900 / 0 = Infinity): zoom from the other alone, accepted.
  assert.deepEqual(fit(0, 300), { x: 400, y: 300, zoom: 2 }); // min(Infinity, 600 / 300)
  assert.deepEqual(fit(400, 0), { x: 400, y: 300, zoom: 2.25 }); // min(900 / 400, Infinity)
  checkCamera(fit(0, 300));
  checkCamera(fit(400, 0));
  // Both 0: zoom Infinity; a negative dimension: zoom below 0; a NaN dimension: zoom NaN;
  // an infinite dimension: zoom 0; a non-finite centre y. Each is rejected.
  assert.equal(fit(0, 0).zoom, Infinity);
  for (const cam of [fit(0, 0), fit(-400, 300), fit(400, NaN), fit(Infinity, 300), fit(400, 300, 400, NaN)]) {
    assert.throws(() => checkCamera(cam), RangeError);
  }
});
