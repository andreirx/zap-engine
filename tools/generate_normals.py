#!/usr/bin/env python3
"""Generate normal maps from sprite atlas images using Sobel operator.

Usage:
    python tools/generate_normals.py public/assets/tiles.png
    python tools/generate_normals.py public/assets/tiles.png --strength 2.0
    python tools/generate_normals.py public/assets/tiles.png -o public/assets/tiles_normals.png
    python tools/generate_normals.py public/assets/tiles.png --source luminance

Reads the input image and computes per-pixel normals from height gradients.
Height can be derived from alpha channel (default) or luminance.
Output is an RGBA normal map where:
    R = nx * 0.5 + 0.5   (tangent-space X)
    G = ny * 0.5 + 0.5   (tangent-space Y)
    B = nz * 0.5 + 0.5   (tangent-space Z, pointing out of the surface)
    A = source alpha      (preserves transparency for atlas masking)

Requires: Pillow (pip install Pillow)
"""

import argparse
import math
import sys
from pathlib import Path

try:
    from PIL import Image
except ImportError:
    print("Error: Pillow is required. Install with: pip install Pillow", file=sys.stderr)
    sys.exit(1)


def sobel_normal_map(img: Image.Image, strength: float, source: str) -> Image.Image:
    """Generate a normal map from an image using the Sobel operator.

    Args:
        img: Input RGBA image.
        strength: Controls the 'bumpiness' of the normals. Higher = more pronounced.
        source: Height source — 'alpha' or 'luminance'.

    Returns:
        RGBA normal map image.
    """
    width, height = img.size
    pixels = img.load()

    # Extract height values
    height_map = [[0.0] * width for _ in range(height)]
    for y in range(height):
        for x in range(width):
            r, g, b, a = pixels[x, y]
            if source == 'alpha':
                height_map[y][x] = a / 255.0
            else:  # luminance
                height_map[y][x] = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0

    # Sobel kernels
    # Gx: [-1 0 1]  Gy: [-1 -2 -1]
    #     [-2 0 2]       [ 0  0  0]
    #     [-1 0 1]       [ 1  2  1]
    normal_map = Image.new('RGBA', (width, height))
    out_pixels = normal_map.load()

    for y in range(height):
        for x in range(width):
            # Sample 3x3 neighborhood (clamp at edges)
            def h(dx: int, dy: int) -> float:
                sx = max(0, min(width - 1, x + dx))
                sy = max(0, min(height - 1, y + dy))
                return height_map[sy][sx]

            # Sobel horizontal (dh/dx)
            gx = (
                -1.0 * h(-1, -1) + 1.0 * h(1, -1)
                + -2.0 * h(-1, 0) + 2.0 * h(1, 0)
                + -1.0 * h(-1, 1) + 1.0 * h(1, 1)
            )

            # Sobel vertical (dh/dy)
            gy = (
                -1.0 * h(-1, -1) + -2.0 * h(0, -1) + -1.0 * h(1, -1)
                + 1.0 * h(-1, 1) + 2.0 * h(0, 1) + 1.0 * h(1, 1)
            )

            # Scale by strength
            gx *= strength
            gy *= strength

            # Normal vector: (-gx, -gy, 1.0), normalized
            nx = -gx
            ny = -gy
            nz = 1.0
            length = math.sqrt(nx * nx + ny * ny + nz * nz)
            nx /= length
            ny /= length
            nz /= length

            # Encode to [0, 255]: n * 0.5 + 0.5 → [0, 1] → [0, 255]
            r_out = int(max(0, min(255, (nx * 0.5 + 0.5) * 255)))
            g_out = int(max(0, min(255, (ny * 0.5 + 0.5) * 255)))
            b_out = int(max(0, min(255, (nz * 0.5 + 0.5) * 255)))

            # Preserve original alpha for atlas masking
            _, _, _, a = pixels[x, y]
            out_pixels[x, y] = (r_out, g_out, b_out, a)

    return normal_map


def main():
    parser = argparse.ArgumentParser(
        description='Generate normal maps from sprite atlas images using Sobel operator.'
    )
    parser.add_argument('input', type=Path, help='Input image path (PNG)')
    parser.add_argument(
        '-o', '--output', type=Path, default=None,
        help='Output path (default: <input>_normals.png)'
    )
    parser.add_argument(
        '--strength', type=float, default=1.0,
        help='Normal strength multiplier (default: 1.0). Higher = more pronounced bumps.'
    )
    parser.add_argument(
        '--source', choices=['alpha', 'luminance'], default='alpha',
        help='Height source: alpha channel (default) or luminance.'
    )

    args = parser.parse_args()

    if not args.input.exists():
        print(f"Error: Input file not found: {args.input}", file=sys.stderr)
        sys.exit(1)

    output = args.output
    if output is None:
        output = args.input.parent / f"{args.input.stem}_normals{args.input.suffix}"

    print(f"Generating normal map: {args.input} -> {output}")
    print(f"  Strength: {args.strength}, Source: {args.source}")

    img = Image.open(args.input).convert('RGBA')
    normal_map = sobel_normal_map(img, args.strength, args.source)
    normal_map.save(output)

    print(f"Done. Output: {output} ({img.size[0]}x{img.size[1]})")


if __name__ == '__main__':
    main()
