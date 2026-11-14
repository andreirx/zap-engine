/// <reference types="node" />
// Tests of the frame header's camera rectangle (HODA-ZE3, ADR-043). Run with
// Node's own test runner:
//   node --test packages/zap-web/src/worker/frame-reader.test.ts
// The imports carry their extension: Node's type stripping resolves only
// extension-qualified imports (ADR-040).

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readCameraRect, readFrameState } from './frame-reader.ts';
import {
  HEADER_CAMERA_HEIGHT,
  HEADER_CAMERA_WIDTH,
  HEADER_CAMERA_X,
  HEADER_CAMERA_Y,
  HEADER_FLOATS,
  HEADER_INSTANCE_COUNT,
  PROTOCOL_VERSION,
  ProtocolLayout,
} from './protocol.ts';

// A small layout; one instance, so readFrameState has something to render.
function frame(camera: [number, number, number, number]): { buf: Float32Array; layout: ProtocolLayout } {
  const layout = new ProtocolLayout(4, 8, 2, 2, 2, 0, 2, 2, 8, 2, 0, 0);
  const buf = new Float32Array(layout.bufferTotalFloats);
  buf[HEADER_INSTANCE_COUNT] = 1;
  [buf[HEADER_CAMERA_X], buf[HEADER_CAMERA_Y], buf[HEADER_CAMERA_WIDTH], buf[HEADER_CAMERA_HEIGHT]] = camera;
  return { buf, layout };
}

test('ZE3 2c: the header is protocol 6 with 38 floats; the camera uses slots 31, 35, 36, 37', () => {
  assert.equal(PROTOCOL_VERSION, 6);
  assert.equal(HEADER_FLOATS, 38);
  assert.deepEqual([HEADER_CAMERA_X, HEADER_CAMERA_Y, HEADER_CAMERA_WIDTH, HEADER_CAMERA_HEIGHT], [31, 35, 36, 37]);
});

test('ZE3 2d: a header with NaN gives cameraRect null', () => {
  const { buf, layout } = frame([NaN, NaN, NaN, NaN]);
  assert.equal(readCameraRect(buf), null);
  assert.equal(readFrameState(buf, layout)?.cameraRect, null);
});

test('ZE3 2e: a header with values gives them', () => {
  const { buf, layout } = frame([480, 270, 960, 540]);
  const expected = { x: 480, y: 270, width: 960, height: 540 };
  assert.deepEqual(readCameraRect(buf), expected);
  assert.deepEqual(readFrameState(buf, layout)?.cameraRect, expected);
});
