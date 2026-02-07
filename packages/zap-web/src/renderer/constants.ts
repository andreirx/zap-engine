// Shared rendering constants — single source of truth for both WebGPU and Canvas 2D.
// Segment colors live here; WebGPU uploads them as a UBO, Canvas 2D reads them directly.

// Segment colors — 13 entries matching SegmentColor enum in Rust (effects.rs).
// Values in linear 0..1 range (pre-HDR multiplier).
export const SEGMENT_COLORS: readonly [number, number, number][] = [
  [1.0, 0.1, 0.05],   // 0  Red
  [1.0, 0.45, 0.0],   // 1  Orange
  [1.0, 0.9, 0.0],    // 2  Yellow
  [0.5, 1.0, 0.0],    // 3  LimeGreen
  [0.0, 1.0, 0.2],    // 4  Green
  [0.0, 1.0, 0.6],    // 5  GreenCyan
  [0.0, 0.9, 1.0],    // 6  Cyan
  [0.0, 0.5, 1.0],    // 7  SkyBlue
  [0.1, 0.1, 1.0],    // 8  Blue
  [0.4, 0.0, 1.0],    // 9  Indigo
  [0.8, 0.0, 1.0],    // 10 Magenta
  [1.0, 0.0, 0.5],    // 11 Pink
  [1.0, 1.0, 1.0],    // 12 White
];

// Pack colors into a Float32Array for WebGPU uniform buffer upload.
// Layout: 13 × vec4<f32> (xyz = rgb, w = 1.0 padding for std140 alignment).
export function packColorsForGPU(): Float32Array {
  const data = new Float32Array(SEGMENT_COLORS.length * 4);
  for (let i = 0; i < SEGMENT_COLORS.length; i++) {
    const [r, g, b] = SEGMENT_COLORS[i];
    data[i * 4] = r;
    data[i * 4 + 1] = g;
    data[i * 4 + 2] = b;
    data[i * 4 + 3] = 1.0;
  }
  return data;
}

// Colors converted to 0-255 sRGB for Canvas 2D rgba() strings.
export const SEGMENT_COLORS_RGB8: readonly [number, number, number][] =
  SEGMENT_COLORS.map(([r, g, b]) => [
    Math.round(r * 255),
    Math.round(g * 255),
    Math.round(b * 255),
  ]);
