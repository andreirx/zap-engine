# examples/dino-level-editor/public/assets/

Sprite assets for the dinosaur level editor example.

## Asset Sources

The PNGs were copied from `/Users/apple/Downloads/dinozaur` into this example so the browser can load them through Vite. `assets.json` maps each used file to a single-cell ZapEngine atlas entry.

## Finish Gate Normalization

ZapEngine sprite instances are square today: one scalar `scale` reaches the shader and Canvas2D fallback. The original `finish-spate.png` and `finish-fata.png` are 128×384, so this folder includes centered transparent 384×384 adapter images:

- `finish-spate-square.png`
- `finish-fata-square.png`

The original files are kept next to them for traceability; the manifest uses the square adapter images so the gate renders at the intended 128×384 visible shape inside a square sprite instance.

## Volcano Hazard Assets

- `vulcan.png` is 1024×1024 and is rendered as an 8×8 tile background object.
- `fireball.png` is a one-tile rotating projectile sprite. Its top-pointing flame is aligned to movement direction in Rust before rendering.
- `smoke.png` is a one-tile trail sprite. The example emits puffs with linearly decreasing alpha over 128 fixed frames.

## Tile Material Assets

- `pamant*.png` files are 128×128 solid ground tiles.
- `lava.png` is a 128×128 non-solid hazard tile; the Rust play logic treats contact as game-over.
- `water.png` is a 128×128 non-solid medium tile; the Rust play logic applies water-specific gravity, drag, and jump impulse while the dinosaur overlaps it.
