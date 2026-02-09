# components/

Component types attached to entities. Follows a "fat entity" model rather than full ECS.

## Files

| File | Purpose |
|------|---------|
| `entity.rs` | `Entity` struct with optional components |
| `sprite.rs` | `SpriteComponent`, `AtlasId`, `BlendMode` |
| `mesh.rs` | `MeshComponent`, `SDFShape`, `SDFColor` |
| `emitter.rs` | `EmitterComponent`, `EmissionMode`, `ColorMode` |
| `layer.rs` | `RenderLayer` enum (Background through UI) |

## Key Types

- **`Entity`**: Fat struct with `pos`, `scale`, `rotation`, `active`, `tag`, `layer`, and optional components (`sprite`, `body`, `emitter`, `mesh`)
- **`SpriteComponent`**: Atlas reference, UV coordinates, alpha, blend mode
- **`MeshComponent`**: SDF shape (Sphere, Capsule, RoundedBox), color, shininess, emissive
- **`EmitterComponent`**: Particle spawner with rate, lifetime, colors, physics
- **`RenderLayer`**: 6 layers for draw ordering (Background=0 ... UI=5)

## Architecture Notes

The fat entity model trades memory efficiency for simplicity. Each entity has slots for all component types (Option fields). This works well for games with hundreds of entities but wouldn't scale to millions.

See ADR-001 for the rationale behind fat entities over full ECS.
