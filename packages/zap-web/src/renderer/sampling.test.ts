/// <reference types="node" />
// Tests of the sampling rule R-SPAN (HODA-ZE4, ADR-041). Run with Node's own test runner:
//   node --test packages/zap-web/src/renderer/sampling.test.ts
// The import of ./sampling.ts carries its extension: Node's type stripping
// resolves only extension-qualified imports. Every expected value is worked by
// hand in the comment beside it; the comparisons are exact.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spanRect } from './sampling.ts';

test('a span of 1 samples the whole cell from its corner', () => {
  assert.deepStrictEqual(spanRect(3, 2, 1), { u0: 3, v0: 2, size: 1 });
});

test('a span of 2 samples a 2x2 block from the cell corner (unchanged)', () => {
  assert.deepStrictEqual(spanRect(3, 2, 2), { u0: 3, v0: 2, size: 2 });
});

test('a span of 0 or below is read as 1', () => {
  // 0 is what a defaulted Rust RenderInstance holds.
  assert.deepStrictEqual(spanRect(3, 2, 0), { u0: 3, v0: 2, size: 1 });
  assert.deepStrictEqual(spanRect(3, 2, -1), { u0: 3, v0: 2, size: 1 });
});

test('a span that is not finite is read as 1', () => {
  assert.deepStrictEqual(spanRect(3, 2, NaN), { u0: 3, v0: 2, size: 1 });
  assert.deepStrictEqual(spanRect(3, 2, Infinity), { u0: 3, v0: 2, size: 1 });
  assert.deepStrictEqual(spanRect(3, 2, -Infinity), { u0: 3, v0: 2, size: 1 });
});

test('a span between 0 and 1 samples the centred square', () => {
  // 64 pixels of content in an 80-pixel cell: span 0.8, inset (1 - 0.8) / 2 = 0.1
  // cell = 8 pixels from each side. 3 + 0.1 and 2 + 0.1 are exactly 3.1 and 2.1
  // in double precision.
  assert.deepStrictEqual(spanRect(3, 2, 0.8), { u0: 3.1, v0: 2.1, size: 0.8 });
});
