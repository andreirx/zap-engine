# systems/

Systems that process entities and produce render/audio output.

## Files

| File | Purpose |
|------|---------|
| `render.rs` | `build_render_buffer()` — sprites sorted by layer/atlas |
| `sdf_render.rs` | `build_sdf_buffer()` — raymarched shapes |
| `debug.rs` | `debug_draw_colliders()` — physics debug visualization |
| `emitter.rs` | `tick_emitters()` — auto-spawn particles from emitters |
| `text.rs` | `spawn_text()`, `despawn_text()` — character entities |
| `vector.rs` | `VectorState` — CPU-tessellated polygons (lyon) |
| `lighting.rs` | `LightState`, `PointLight` — dynamic 2D lighting |

## Subdirectory

| Directory | Purpose |
|-----------|---------|
| `effects/` | Particle/arc system (split into 7 submodules) |

## Effects Submodules

| File | Purpose |
|------|---------|
| `rng.rs` | Xorshift64 RNG |
| `segment_color.rs` | Arc color UV lookup |
| `geometry.rs` | Triangle strip generation |
| `electric_arc.rs` | Midpoint-displacement arcs |
| `particle.rs` | Particle physics + rendering |
| `debug_line.rs` | Debug line strip |
| `mod.rs` | `EffectsState` facade |

## Draw Order

```
1. Sprites (sorted by layer, then atlas)
2. Vectors (CPU-tessellated polygons)
3. SDF shapes (raymarched spheres, capsules, boxes)
4. Effects (additive particles and arcs)
```

## Architecture Notes

Systems are free functions or structs that transform data. They avoid holding references to `Scene` or `EngineContext` during iteration to prevent borrow conflicts.

`build_render_buffer()` sorts entities by `(layer, atlas)` and outputs `LayerBatch` descriptors for the renderer.
