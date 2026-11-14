// Which part of the atlas a sprite instance samples (R-SPAN, ADR-041).
//
// An instance names a sheet cell (col, row) and a `cell_span`. This function
// is the one statement of the rule; the sprite vertex shader (shaders.wgsl,
// vs_main) and the Canvas2D fallback (canvas2d.ts, drawInstance) transcribe it.
// A change here is a change to both, in the same commit.

/** A square of the atlas in sheet-cell units: origin (u0, v0), side `size`. */
export interface SpanRect {
  u0: number;
  v0: number;
  size: number;
}

/**
 * The square of the atlas that the instance at sheet cell (col, row) samples.
 *
 * - `cellSpan` ≤ 0 or not finite: read as 1 (the whole cell). 0 is what a
 *   defaulted Rust `RenderInstance` holds.
 * - 0 < `cellSpan` < 1: the square of side `cellSpan`, centred in the cell
 *   (a picture with a bleed margin drawn at its content's size).
 * - `cellSpan` ≥ 1: `cellSpan` × `cellSpan` cells from the cell's corner.
 */
export function spanRect(col: number, row: number, cellSpan: number): SpanRect {
  if (!Number.isFinite(cellSpan) || cellSpan <= 0) {
    return { u0: col, v0: row, size: 1 };
  }
  if (cellSpan < 1) {
    const inset = (1 - cellSpan) / 2;
    return { u0: col + inset, v0: row + inset, size: cellSpan };
  }
  return { u0: col, v0: row, size: cellSpan };
}
