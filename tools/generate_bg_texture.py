#!/usr/bin/env python3
"""Generate a dark parchment/leather background texture using value noise.

Usage:
    python tools/generate_bg_texture.py
    python tools/generate_bg_texture.py --size 512 --output examples/glypher/public/assets/bg_1x1.png

Produces a tileable dark texture suitable for the Glypher game background.
Uses layered value noise (no external dependencies beyond Pillow).

Requires: Pillow (pip install Pillow)
"""

import argparse
import math
import random
import sys
from pathlib import Path

try:
    from PIL import Image
except ImportError:
    print("Error: Pillow is required. Install with: pip install Pillow", file=sys.stderr)
    sys.exit(1)


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def smoothstep(t: float) -> float:
    return t * t * (3.0 - 2.0 * t)


class ValueNoise2D:
    """Simple 2D value noise with smooth interpolation."""

    def __init__(self, seed: int = 42):
        rng = random.Random(seed)
        self.perm = list(range(256))
        rng.shuffle(self.perm)
        self.perm += self.perm  # Double for wrapping
        self.values = [rng.random() for _ in range(256)]

    def _hash(self, ix: int, iy: int) -> float:
        return self.values[self.perm[self.perm[ix & 255] + (iy & 255)] & 255]

    def sample(self, x: float, y: float) -> float:
        ix = int(math.floor(x))
        iy = int(math.floor(y))
        fx = x - ix
        fy = y - iy
        fx = smoothstep(fx)
        fy = smoothstep(fy)

        v00 = self._hash(ix, iy)
        v10 = self._hash(ix + 1, iy)
        v01 = self._hash(ix, iy + 1)
        v11 = self._hash(ix + 1, iy + 1)

        return lerp(lerp(v00, v10, fx), lerp(v01, v11, fx), fy)


def fbm(noise: ValueNoise2D, x: float, y: float, octaves: int = 6) -> float:
    """Fractal Brownian Motion — layered noise for natural textures."""
    value = 0.0
    amplitude = 0.5
    frequency = 1.0
    for _ in range(octaves):
        value += amplitude * noise.sample(x * frequency, y * frequency)
        amplitude *= 0.5
        frequency *= 2.0
    return value


def generate_texture(size: int, seed: int) -> Image.Image:
    """Generate a dark parchment-like background texture."""
    noise1 = ValueNoise2D(seed)
    noise2 = ValueNoise2D(seed + 137)
    noise3 = ValueNoise2D(seed + 271)

    img = Image.new('RGBA', (size, size))
    pixels = img.load()

    # Dark parchment/leather base colors
    base_r, base_g, base_b = 35, 30, 28
    # Slight warm variation
    var_r, var_g, var_b = 20, 15, 12

    for y in range(size):
        for x in range(size):
            # Tileable coordinates (wrap at edges)
            nx = x / size * 8.0
            ny = y / size * 8.0

            # Large-scale tone variation
            f1 = fbm(noise1, nx, ny, 5)
            # Medium grain
            f2 = fbm(noise2, nx * 2.0, ny * 2.0, 4)
            # Fine grain
            f3 = fbm(noise3, nx * 6.0, ny * 6.0, 3)

            # Combine: base + variation weighted by noise layers
            v = f1 * 0.5 + f2 * 0.3 + f3 * 0.2
            v = max(0.0, min(1.0, v))

            r = int(max(0, min(255, base_r + var_r * v)))
            g = int(max(0, min(255, base_g + var_g * v)))
            b = int(max(0, min(255, base_b + var_b * v)))

            pixels[x, y] = (r, g, b, 255)

    return img


def main():
    parser = argparse.ArgumentParser(
        description='Generate a dark parchment background texture using value noise.'
    )
    parser.add_argument(
        '--size', type=int, default=512,
        help='Texture size in pixels (default: 512)'
    )
    parser.add_argument(
        '--seed', type=int, default=42,
        help='Random seed (default: 42)'
    )
    parser.add_argument(
        '-o', '--output', type=Path,
        default=Path('examples/glypher/public/assets/bg_1x1.png'),
        help='Output path (default: examples/glypher/public/assets/bg_1x1.png)'
    )

    args = parser.parse_args()

    print(f"Generating {args.size}x{args.size} background texture (seed={args.seed})")
    img = generate_texture(args.size, args.seed)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    img.save(args.output)
    print(f"Done: {args.output}")


if __name__ == '__main__':
    main()
