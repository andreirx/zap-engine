# bridge/

SharedArrayBuffer wire protocol connecting Rust (WASM) to TypeScript (renderer).

## Files

| File | Purpose |
|------|---------|
| `protocol.rs` | `ProtocolLayout`, header constants, offset calculations |

## Wire Format

The SharedArrayBuffer is a contiguous `Float32Array` with self-describing header:

```
[Header: 28 floats]
  0: lock (Atomics)      1: frame_counter       2: max_instances
  3: instance_count      4: atlas_split         5: max_effects_vertices
  6: effects_count       7: world_width         8: world_height
  9: max_sounds         10: sound_count        11: max_events
 12: event_count        13: protocol_version   14: max_sdf_instances
 15: sdf_count          16: max_vector_verts   17: vector_count
 18: max_layer_batches  19: layer_batch_count  20: layer_batch_offset
 21: bake_state         22: max_lights         23: light_count
 24: ambient_r          25: ambient_g          26: ambient_b
 27: reserved

[Instances: max_instances × 8 floats]
[Effects: max_effects_vertices × 5 floats]
[Sounds: max_sounds × 1 float]
[Events: max_events × 4 floats]
[SDF: max_sdf_instances × 12 floats]
[Vectors: max_vector_vertices × 6 floats]
[LayerBatches: max_layer_batches × 4 floats]
[Lights: max_lights × 8 floats]
```

## Architecture Notes

`ProtocolLayout` computes all offsets from `GameConfig` capacities. TypeScript reads capacities from the header to construct a matching `ProtocolLayout`, enabling forward compatibility when the protocol evolves.

Protocol version is currently 4.0 (Phase 9 lighting).
