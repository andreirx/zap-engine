# tools/

Build-time utilities for the ZapEngine workflow.

## Files

| Tool | Purpose |
|---|---|
| `bake-assets.ts` | CLI: scans image directories, generates `assets.json` manifests |
| `font-atlas-generator.html` | Browser: renders font characters into a grid atlas PNG |
| `glyph-editor.html` | Browser: visual editor for defining letter stroke paths (used by Glypher game) |
| `generate_normals.py` | CLI: generates normal maps from atlas PNGs using Sobel operator |

## bake-assets.ts

Scans an image directory and generates an `assets.json` manifest compatible with the engine's `AssetManifest` format.

**Convention-based atlas detection:**
- `hero_4x8.png` -> atlas "hero" with 4 columns, 8 rows -> sprites `hero_0_0` through `hero_3_7`
- `background.png` -> single-sprite atlas -> sprite `background`

**Usage:**
```bash
npx tsx tools/bake-assets.ts path/to/images/ --output path/to/assets.json
```

## glyph-editor.html

Vector path editor for defining letter stroke paths with bezier curves. Open directly in a browser (no build step). Similar workflow to Photoshop/Illustrator pen tool.

**Two tools:**
- **Draw tool**: Click to place anchor points as a polyline. Faint cursor line tracks from last point to mouse. Right-click or Enter to commit — polyline converts to smooth bezier curves via Catmull-Rom tangent estimation.
- **Edit tool**: Click anchor points to select them. Drag to reposition. Selected points reveal purple (handle-in) and orange (handle-out) bezier control handles. Drag handles to reshape curves per-segment. Delete key removes selected point.

**Type-aware variant system:**
- **Lowercase (a-z)**: Two entry variants — Baseline and High (contextual alternates). Exit type is hardcoded per letter (b, o, v, w → High; all others → Baseline). The game uses exit type to choose the next letter's entry variant.
- **Uppercase (A-Z)** and **Digits (0-9)**: Single "Default" variant, no entry/exit system. Variant dropdown is disabled.

**Letter width system (hardcoded):**
| Width | Characters |
|-------|-----------|
| 0 (narrow) | i, j, l, t, e, f, I, 1 |
| 1 (standard) | most letters and digits |
| 2 (wide) | m, w, M, W |

Width determines vertical guideline layout on the canvas:
- Width 0: one center vertical, entry/exit zones straddling it
- Width 1: two verticals at 0.20 and 0.80
- Width 2: three verticals at 0.10, 0.50, 0.90

**Canvas guides:**
- **Horizontal**: ascender (0.0), cap (0.15), x-height (0.40), baseline (0.75), descender (1.0)
- **Vertical**: teal dashed lines based on letter width
- **Entry boxes** (lowercase only): cyan (Baseline) and magenta (High) boxes at entry position — active variant is filled, other is dim
- **Exit box** (lowercase only): orange box at exit position — height depends on exit type (Baseline → baseline, High → x-height)

**Features:**
- Lowercase (a-z), uppercase (A-Z), and digit (0-9) support
- **Stroke list panel**: committed strokes listed with point counts, reorderable (up/down), deletable
- Stroke direction arrows and stroke order numbers on canvas
- Ghost rendering of the other variant's strokes (lowercase only)
- Shift+click snaps Y to nearest horizontal guide
- localStorage auto-persistence with migration from old formats
- Two file download exports:
  - **Edit JSON**: anchor points with bezier handles `{x, y, hi, ho}`, `exit` and `width` per glyph (for re-importing and continuing work)
  - **Baked JSON**: `{ meta: { highExitLetters, widths }, glyphs: {...} }` — 20 interpolated points per bezier segment (for game consumption)
- Import supports edit format, baked format (auto-unwraps `meta`+`glyphs`), and legacy `[[x,y],...]` format (auto-converts). Migrates old Baseline/High variants to Default for uppercase/digits.
- Keyboard: letter/digit keys to switch, Ctrl+Z undo, Enter commit, Escape deselect/discard, Delete remove point/stroke

**Output:** JSON mapping each character to its stroke paths, consumed by the Glypher game. Baked format preserves stroke order, direction, entry variants, exit types, and widths for in-game contextual alternate selection and guide light tracing.

## Architecture Connection

The baker's output feeds into `loadManifest()` (TypeScript) and the Rust `AssetManifest` parser. The schema is defined in `packages/zap-web/src/assets/manifest.ts` and `crates/zap-engine/src/assets/manifest.rs`.
