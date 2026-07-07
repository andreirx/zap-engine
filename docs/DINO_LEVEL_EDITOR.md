# Dino Level Editor Example

## Purpose

`examples/dino-level-editor/` demonstrates a small, playable, editor-driven platformer built on ZapEngine. It is intentionally an example-local implementation: one game owns the tile semantics, player motion, and UI commands, so introducing a generic editor package would add maintenance surface before there is demonstrated variation.

## Data Model

- The level is a fixed 64×32 tile grid.
- Terrain is stored as a tile material plus a derived ground sprite kind.
- Money is stored as small map objects on tile cells with value `1` or `10`.
- Powerups are stored as small map objects on tile cells. The current powerup is `carne`, which grants the temporary big-dinosaur state.
- The dinosaur start position is continuous world-space, constrained to the terrain surface.
- The finish gate is tile-positioned and split into back/front draw layers.
- The volcano is movable background scenery. Fireballs are deterministic local hazard state rather than a reusable engine system.

## Level Persistence

Level persistence is split along the ZapEngine headless boundary:

- Rust owns the authoritative level document because Rust owns editor state, collision state, player placement, and hazard placement.
- React owns browser storage because IndexedDB/localStorage are browser APIs and belong on the host/UI side. IndexedDB is preferred; localStorage is a fallback for browser shells that do not expose IndexedDB; an in-memory fallback keeps restricted browser shells operable for the current page session.
- The worker boundary passes raw JSON strings through the existing `export_world`/`world_export` and `load_level` hooks.

The saved document is versioned (`version: 1`) and uses simple DTO fields: tile material codes, money objects, powerup objects, dino start/color, finish tile, and volcano tile. Loading is intentionally tolerant: missing fields fall back to the starter level, the older `ground: bool[]` shape is accepted for ground-only saves, missing `powerups` defaults to none, unknown tile/color codes become safe defaults, and out-of-range positions are clamped.

The default slot is treated as editor continuity rather than an explicit user artifact: React auto-loads it once the engine is ready and auto-saves it while in edit mode. The Save/Load buttons are therefore reserved for named slots. Named slots share the same versioned Rust JSON document and browser-storage backend as the default slot.

## Tile Materials

The editor keeps material semantics local to the example because there is one current caller and no engine-wide terrain contract yet.

- `Ground` is the only solid tile material. Its rendered sprite is derived from topology: exposed top cells use `pamant`, covered cells use deterministic underground variations.
- `Lava` is non-solid so the dinosaur can overlap it, but any overlap in play mode immediately returns to edit mode.
- `Water` is non-solid and uses a separate movement branch: lower gravity, horizontal/vertical drag, capped sinking speed, and a per-keypress upward impulse. If the dinosaur leaves water while moving upward, that velocity carries into air, which creates the requested “slow in water, faster in air” transition without a special-case launch state.

## Rendering Contract

The current renderer wire format sends square sprite instances: position, rotation, scalar scale, atlas cell, alpha, and layer batch metadata. Because the finish gate artwork is 128×384, the example uses transparent 384×384 adapter images for the front/back gate halves. The actual visible pixels remain 128×384, centered inside the square texture.

The powered-up dinosaur uses `dino-*-big-*` square adapter sprites. Each adapter scales the original 128×128 frame to 256×256, so the visible dinosaur and its collision box both double in width and height. This is deliberately example-local because the current requirement is a normal uniform scale-up, not a ratified renderer protocol change.

## Camera Contract

The example keeps logical level coordinates separate from render-plane coordinates. The Rust game computes an example-local camera transform and spawns entities in the fixed 1600×900 render plane. This avoids changing engine protocol for a single example while still showing:

- edit mode fit-to-level view,
- play mode zoom around the character,
- animated interpolation between the two,
- forward-only play camera progression.

## Volcano Hazard

`vulcan.png` is rendered behind terrain at 8×8 tiles. The editor's Volcano tool moves it as a tile-aligned 8×8 object by clicking the desired center point. Four small fireballs loop around the crater in both edit and play mode. Larger eruptions launch every 0–2 seconds independently of existing airborne fireballs, so several can be in flight at once. Each eruption travels through a quadratic arc toward a random level X coordinate and disappears when its sprite bounds touch any tile material (`Ground`, `Lava`, or `Water`) or when its path completes. In edit mode it is visual-only. In play mode, contact with a small dinosaur returns to edit mode as game-over behavior; contact with a powered-up dinosaur consumes the extra life, removes the big state, and removes that fireball.

Smoke trails are explicit sprite puffs because the user-facing requirement is tied to `smoke.png` and a 128-frame alpha fade. Each puff stores position, rotation, size, and frame age; render alpha is `1 - age / 128`.

Fireballs render as additive sprites with a 4× shader alpha multiplier. This uses the existing HDR sprite path rather than mutating the source PNGs, preserving the intentionally dark painted base while letting the light accents bloom.

## Controls

- Edit mode: toolbar selects tool, place/erase, and piecewise vs rectangle editing.
- Tile tools include Ground, Lava, and Water.
- Object tools include money and `carne` powerups. In play mode, collecting `carne` doubles dinosaur width and height and grants one fireball hit buffer.
- Edit mode also exposes named Save, named Load, and Reset. The default slot auto-loads and auto-saves; Reset is guarded by a Yes/Cancel dialog and will be auto-saved if the editor remains open.
- Rectangle mode updates `last_cell` during drag without mutating the level until pointer-up, then renders a vector overlay preview from `rect_start` to `last_cell`.
- Play mode: `D`/right arrow moves forward, `Space`/`W`/up arrow jumps. Standing and running jumps intentionally use the same impulse based on play-tester feedback.
- Finish or falling below the map returns to edit mode.
