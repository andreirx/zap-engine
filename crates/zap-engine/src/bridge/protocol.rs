/// SharedArrayBuffer layout.
/// Must stay in sync with TypeScript `protocol.ts`.
///
/// Layout (all values in f32 / 4 bytes):
/// ```text
/// [Header: 28 floats]
/// [Instances: max_instances × 8 floats]
/// [Effects: max_effects_vertices × 5 floats]
/// [Sounds: max_sounds × 1 float]
/// [Events: max_events × 4 floats]
/// [SDF: max_sdf_instances × 12 floats]
/// [Vectors: max_vector_vertices × 6 floats]
/// [LayerBatches: max_layer_batches × 4 floats]
/// [Lights: max_lights × 8 floats]
/// ```
///
/// Capacities are written once into the header at init.
/// TypeScript reads them from the header to compute offsets dynamically.

use crate::api::game::GameConfig;

/// Number of floats in the header section.
pub const HEADER_FLOATS: usize = 28;

/// Header field indices.
pub const HEADER_LOCK: usize = 0;
pub const HEADER_FRAME_COUNTER: usize = 1;
pub const HEADER_MAX_INSTANCES: usize = 2;
pub const HEADER_INSTANCE_COUNT: usize = 3;
pub const HEADER_ATLAS_SPLIT: usize = 4;
pub const HEADER_MAX_EFFECTS_VERTICES: usize = 5;
pub const HEADER_EFFECTS_VERTEX_COUNT: usize = 6;
pub const HEADER_WORLD_WIDTH: usize = 7;
pub const HEADER_WORLD_HEIGHT: usize = 8;
pub const HEADER_MAX_SOUNDS: usize = 9;
pub const HEADER_SOUND_COUNT: usize = 10;
pub const HEADER_MAX_EVENTS: usize = 11;
pub const HEADER_EVENT_COUNT: usize = 12;
pub const HEADER_PROTOCOL_VERSION: usize = 13;
pub const HEADER_MAX_SDF_INSTANCES: usize = 14;
pub const HEADER_SDF_INSTANCE_COUNT: usize = 15;
pub const HEADER_MAX_VECTOR_VERTICES: usize = 16;
pub const HEADER_VECTOR_VERTEX_COUNT: usize = 17;
// Phase 8: Layer batches
pub const HEADER_MAX_LAYER_BATCHES: usize = 18;
pub const HEADER_LAYER_BATCH_COUNT: usize = 19;
pub const HEADER_LAYER_BATCH_OFFSET: usize = 20;
/// Encoded bake state: `baked_layers_mask | (bake_generation << 6)`.
pub const HEADER_BAKE_STATE: usize = 21;
// Phase 9: Dynamic lighting
pub const HEADER_MAX_LIGHTS: usize = 22;
pub const HEADER_LIGHT_COUNT: usize = 23;
pub const HEADER_AMBIENT_R: usize = 24;
pub const HEADER_AMBIENT_G: usize = 25;
pub const HEADER_AMBIENT_B: usize = 26;
pub const HEADER_RESERVED_27: usize = 27;

/// Protocol version written into the header.
pub const PROTOCOL_VERSION: f32 = 4.0;

/// Floats per render instance (wire format — never changes).
pub const INSTANCE_FLOATS: usize = 8;

/// Floats per effects vertex: x, y, z, u, v (wire format — never changes).
pub const EFFECTS_VERTEX_FLOATS: usize = 5;

/// Floats per game event: kind, a, b, c (wire format — never changes).
pub const EVENT_FLOATS: usize = 4;

/// Floats per SDF instance: x, y, radius, rotation, r, g, b, shininess, emissive, shape_type, half_height, extra.
pub const SDF_INSTANCE_FLOATS: usize = 12;

/// Floats per vector vertex: x, y, r, g, b, a (wire format — never changes).
pub const VECTOR_VERTEX_FLOATS: usize = 6;

/// Floats per layer batch descriptor: layer_id, start, end, atlas_id.
pub const LAYER_BATCH_FLOATS: usize = 4;

/// Floats per point light: x, y, r, g, b, intensity, radius, layer_mask.
pub const LIGHT_FLOATS: usize = 8;

/// Default maximum layer batches (one per (layer, atlas) pair).
/// With 6 layers and up to 8 atlases, 48 is a reasonable default.
pub const DEFAULT_MAX_LAYER_BATCHES: usize = 48;

/// Default maximum point lights.
pub const DEFAULT_MAX_LIGHTS: usize = 64;

/// Runtime-computed buffer layout. Replaces the old compile-time MAX_* constants.
#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolLayout {
    /// Maximum render instances.
    pub max_instances: usize,
    /// Maximum effects vertices.
    pub max_effects_vertices: usize,
    /// Maximum sound events per frame.
    pub max_sounds: usize,
    /// Maximum game events per frame.
    pub max_events: usize,
    /// Maximum SDF instances.
    pub max_sdf_instances: usize,
    /// Maximum vector vertices.
    pub max_vector_vertices: usize,
    /// Maximum layer batches.
    pub max_layer_batches: usize,
    /// Maximum point lights.
    pub max_lights: usize,

    /// Size of instance data section in floats.
    pub instance_data_floats: usize,
    /// Size of effects data section in floats.
    pub effects_data_floats: usize,
    /// Size of sound data section in floats.
    pub sound_data_floats: usize,
    /// Size of event data section in floats.
    pub event_data_floats: usize,
    /// Size of SDF data section in floats.
    pub sdf_data_floats: usize,
    /// Size of vector data section in floats.
    pub vector_data_floats: usize,
    /// Size of layer batch data section in floats.
    pub layer_batch_data_floats: usize,
    /// Size of light data section in floats.
    pub light_data_floats: usize,

    /// Offset (in floats) where instance data begins.
    pub instance_data_offset: usize,
    /// Offset (in floats) where effects data begins.
    pub effects_data_offset: usize,
    /// Offset (in floats) where sound data begins.
    pub sound_data_offset: usize,
    /// Offset (in floats) where event data begins.
    pub event_data_offset: usize,
    /// Offset (in floats) where SDF data begins.
    pub sdf_data_offset: usize,
    /// Offset (in floats) where vector data begins.
    pub vector_data_offset: usize,
    /// Offset (in floats) where layer batch data begins.
    pub layer_batch_data_offset: usize,
    /// Offset (in floats) where light data begins.
    pub light_data_offset: usize,

    /// Total buffer size in floats.
    pub buffer_total_floats: usize,
    /// Total buffer size in bytes.
    pub buffer_total_bytes: usize,
}

impl ProtocolLayout {
    /// Compute layout from raw capacity values.
    pub fn new(
        max_instances: usize,
        max_effects_vertices: usize,
        max_sounds: usize,
        max_events: usize,
        max_sdf_instances: usize,
        max_vector_vertices: usize,
        max_layer_batches: usize,
        max_lights: usize,
    ) -> Self {
        let instance_data_floats = max_instances * INSTANCE_FLOATS;
        let effects_data_floats = max_effects_vertices * EFFECTS_VERTEX_FLOATS;
        let sound_data_floats = max_sounds;
        let event_data_floats = max_events * EVENT_FLOATS;
        let sdf_data_floats = max_sdf_instances * SDF_INSTANCE_FLOATS;
        let vector_data_floats = max_vector_vertices * VECTOR_VERTEX_FLOATS;
        let layer_batch_data_floats = max_layer_batches * LAYER_BATCH_FLOATS;
        let light_data_floats = max_lights * LIGHT_FLOATS;

        let instance_data_offset = HEADER_FLOATS;
        let effects_data_offset = instance_data_offset + instance_data_floats;
        let sound_data_offset = effects_data_offset + effects_data_floats;
        let event_data_offset = sound_data_offset + sound_data_floats;
        let sdf_data_offset = event_data_offset + event_data_floats;
        let vector_data_offset = sdf_data_offset + sdf_data_floats;
        let layer_batch_data_offset = vector_data_offset + vector_data_floats;
        let light_data_offset = layer_batch_data_offset + layer_batch_data_floats;

        let buffer_total_floats = light_data_offset + light_data_floats;
        let buffer_total_bytes = buffer_total_floats * 4;

        Self {
            max_instances,
            max_effects_vertices,
            max_sounds,
            max_events,
            max_sdf_instances,
            max_vector_vertices,
            max_layer_batches,
            max_lights,
            instance_data_floats,
            effects_data_floats,
            sound_data_floats,
            event_data_floats,
            sdf_data_floats,
            vector_data_floats,
            layer_batch_data_floats,
            light_data_floats,
            instance_data_offset,
            effects_data_offset,
            sound_data_offset,
            event_data_offset,
            sdf_data_offset,
            vector_data_offset,
            layer_batch_data_offset,
            light_data_offset,
            buffer_total_floats,
            buffer_total_bytes,
        }
    }

    /// Compute layout from a GameConfig.
    #[cfg(feature = "vectors")]
    pub fn from_config(config: &GameConfig) -> Self {
        Self::new(
            config.max_instances,
            config.max_effects_vertices,
            config.max_sounds,
            config.max_events,
            config.max_sdf_instances,
            config.max_vector_vertices,
            config.max_layer_batches,
            config.max_lights,
        )
    }

    /// Compute layout from a GameConfig (without vectors).
    #[cfg(not(feature = "vectors"))]
    pub fn from_config(config: &GameConfig) -> Self {
        Self::new(
            config.max_instances,
            config.max_effects_vertices,
            config.max_sounds,
            config.max_events,
            config.max_sdf_instances,
            0, // No vector vertices when vectors feature is disabled
            config.max_layer_batches,
            config.max_lights,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Default values for testing (must match GameConfig::default())
    const DEFAULT_MAX_INSTANCES: usize = 2048;
    const DEFAULT_MAX_EFFECTS_VERTICES: usize = 16384;
    const DEFAULT_MAX_SOUNDS: usize = 32;
    const DEFAULT_MAX_EVENTS: usize = 32;
    const DEFAULT_MAX_SDF_INSTANCES: usize = 256;
    #[cfg(feature = "vectors")]
    const DEFAULT_MAX_VECTOR_VERTICES: usize = 16384;
    #[cfg(not(feature = "vectors"))]
    const DEFAULT_MAX_VECTOR_VERTICES: usize = 0;
    const TEST_MAX_LIGHTS: usize = 64;

    #[test]
    fn from_default_config_matches_expected_sizes() {
        let layout = ProtocolLayout::from_config(&GameConfig::default());

        assert_eq!(layout.max_instances, DEFAULT_MAX_INSTANCES);
        assert_eq!(layout.max_effects_vertices, DEFAULT_MAX_EFFECTS_VERTICES);
        assert_eq!(layout.max_sounds, DEFAULT_MAX_SOUNDS);
        assert_eq!(layout.max_events, DEFAULT_MAX_EVENTS);
        assert_eq!(layout.max_sdf_instances, DEFAULT_MAX_SDF_INSTANCES);
        assert_eq!(layout.max_vector_vertices, DEFAULT_MAX_VECTOR_VERTICES);
        assert_eq!(layout.max_layer_batches, DEFAULT_MAX_LAYER_BATCHES);
        assert_eq!(layout.max_lights, TEST_MAX_LIGHTS);
    }

    #[test]
    fn custom_capacities_compute_correctly() {
        let layout = ProtocolLayout::new(256, 8192, 16, 64, 64, 4096, 8, 32);

        assert_eq!(layout.instance_data_floats, 256 * 8);
        assert_eq!(layout.effects_data_floats, 8192 * 5);
        assert_eq!(layout.sound_data_floats, 16);
        assert_eq!(layout.event_data_floats, 64 * 4);
        assert_eq!(layout.sdf_data_floats, 64 * 12);
        assert_eq!(layout.vector_data_floats, 4096 * 6);
        assert_eq!(layout.layer_batch_data_floats, 8 * 4);
        assert_eq!(layout.light_data_floats, 32 * 8);

        let expected_total = HEADER_FLOATS
            + 256 * 8
            + 8192 * 5
            + 16
            + 64 * 4
            + 64 * 12
            + 4096 * 6
            + 8 * 4
            + 32 * 8;
        assert_eq!(layout.buffer_total_floats, expected_total);
        assert_eq!(layout.buffer_total_bytes, expected_total * 4);
    }

    #[test]
    fn offsets_are_contiguous() {
        let layout = ProtocolLayout::new(100, 200, 10, 20, 50, 100, 6, 16);

        assert_eq!(layout.instance_data_offset, HEADER_FLOATS);
        assert_eq!(layout.effects_data_offset, layout.instance_data_offset + layout.instance_data_floats);
        assert_eq!(layout.sound_data_offset, layout.effects_data_offset + layout.effects_data_floats);
        assert_eq!(layout.event_data_offset, layout.sound_data_offset + layout.sound_data_floats);
        assert_eq!(layout.sdf_data_offset, layout.event_data_offset + layout.event_data_floats);
        assert_eq!(layout.vector_data_offset, layout.sdf_data_offset + layout.sdf_data_floats);
        assert_eq!(layout.layer_batch_data_offset, layout.vector_data_offset + layout.vector_data_floats);
        assert_eq!(layout.light_data_offset, layout.layer_batch_data_offset + layout.layer_batch_data_floats);
        assert_eq!(layout.buffer_total_floats, layout.light_data_offset + layout.light_data_floats);
    }

    #[test]
    fn header_size_is_28() {
        assert_eq!(HEADER_FLOATS, 28);
        assert_eq!(HEADER_MAX_LAYER_BATCHES, 18);
        assert_eq!(HEADER_LAYER_BATCH_COUNT, 19);
        assert_eq!(HEADER_LAYER_BATCH_OFFSET, 20);
        assert_eq!(HEADER_MAX_LIGHTS, 22);
        assert_eq!(HEADER_LIGHT_COUNT, 23);
        assert_eq!(HEADER_AMBIENT_R, 24);
    }

    #[test]
    fn light_section_comes_after_layer_batches() {
        let layout = ProtocolLayout::new(512, 16384, 32, 32, 128, 8192, 6, 64);
        assert_eq!(layout.light_data_offset, layout.layer_batch_data_offset + layout.layer_batch_data_floats);
    }

    #[test]
    fn protocol_version_is_4() {
        assert_eq!(PROTOCOL_VERSION, 4.0);
    }
}
