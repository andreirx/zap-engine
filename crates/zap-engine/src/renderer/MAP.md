# renderer/

Render data structures prepared by Rust for consumption by TypeScript renderers.

## Files

| File | Purpose |
|------|---------|
| `camera.rs` | `Camera2D` with orthographic projection |
| `instance.rs` | `RenderInstance` (8 floats), `RenderBuffer` |
| `sdf_instance.rs` | `SDFInstance` (12 floats), `SDFBuffer` |

## Wire Formats

**RenderInstance (32 bytes):**
```
x, y, rotation, scale, sprite_col, alpha, cell_span, atlas_row
```

**SDFInstance (48 bytes):**
```
x, y, radius, rotation, r, g, b, shininess, emissive, shape_type, half_height, extra
```

## Key Types

- **`Camera2D`**: World dimensions + orthographic projection matrix
- **`RenderInstance`**: Per-sprite data for the GPU (position, rotation, UV coords)
- **`SDFInstance`**: Per-shape data for raymarched SDF rendering

## Architecture Notes

These are pure data structures — no GPU code lives in `zap-engine`. The actual WebGPU/Canvas2D rendering is in `packages/zap-web/src/renderer/`.

`scale` is world-space size (not a multiplier). The shader uses it directly.

The `#[repr(C)]` attribute ensures struct layout matches the SharedArrayBuffer format.
