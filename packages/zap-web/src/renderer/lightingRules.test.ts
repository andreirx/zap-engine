/// <reference types="node" />
// Tests of the normal-buffer rule R-FLAT-NORMAL (HODA-ZE5, ADR-042). Run with
// Node's own test runner:
//   node --test packages/zap-web/src/renderer/lightingRules.test.ts
// The import of ./lightingRules.ts carries its extension: Node's type stripping
// resolves only extension-qualified imports.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { drawsSpriteNormals, normalPassRuns } from './lightingRules.ts';

test('lighting off: the normal-buffer pass does not run, with or without a buffer', () => {
  assert.equal(normalPassRuns(false, false), false);
  assert.equal(normalPassRuns(false, true), false);
});

test('lighting on with a normal buffer: the pass runs (normal maps are not an input)', () => {
  assert.equal(normalPassRuns(true, true), true);
});

test('lighting on without a normal buffer: the pass does not run', () => {
  assert.equal(normalPassRuns(true, false), false);
});

test('sprite normals are drawn only with normal maps', () => {
  assert.equal(drawsSpriteNormals(true), true);
  assert.equal(drawsSpriteNormals(false), false);
});
