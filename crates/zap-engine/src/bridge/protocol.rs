/// SharedArrayBuffer layout.
/// Must stay in sync with TypeScript `protocol.ts`.
///
/// Layout (all values in f32 / 4 bytes):
/// ```text
/// [Header: 16 floats]
/// [Instances: max_instances × 8 floats]
/// [Effects: max_effects_vertices × 5 floats]
/// [Sounds: max_sounds × 1 float]
/// [Events: max_events × 4 floats]
/// ```
///
/// Capacities are written once into the header at init.
/// TypeScript reads them from the header to compute offsets dynamically.

use crate::api::game::GameConfig;

/// Number of floats in the header section.
pub const HEADER_FLOATS: usize = 16;

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

/// Protocol version written into the header.
pub const PROTOCOL_VERSION: f32 = 1.0;

/// Floats per render instance (wire format — never changes).
pub const INSTANCE_FLOATS: usize = 8;

/// Floats per effects vertex: x, y, z, u, v (wire format — never changes).
pub const EFFECTS_VERTEX_FLOATS: usize = 5;

/// Floats per game event: kind, a, b, c (wire format — never changes).
pub const EVENT_FLOATS: usize = 4;

/// Floats per SDF instance: x, y, radius, rotation, r, g, b, shininess, emissive, pad×3.
pub const SDF_INSTANCE_FLOATS: usize = 12;

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
    ) -> Self {
        let instance_data_floats = max_instances * INSTANCE_FLOATS;
        let effects_data_floats = max_effects_vertices * EFFECTS_VERTEX_FLOATS;
        let sound_data_floats = max_sounds;
        let event_data_floats = max_events * EVENT_FLOATS;
        let sdf_data_floats = max_sdf_instances * SDF_INSTANCE_FLOATS;

        let instance_data_offset = HEADER_FLOATS;
        let effects_data_offset = instance_data_offset + instance_data_floats;
        let sound_data_offset = effects_data_offset + effects_data_floats;
        let event_data_offset = sound_data_offset + sound_data_floats;
        let sdf_data_offset = event_data_offset + event_data_floats;

        let buffer_total_floats = sdf_data_offset + sdf_data_floats;
        let buffer_total_bytes = buffer_total_floats * 4;

        Self {
            max_instances,
            max_effects_vertices,
            max_sounds,
            max_events,
            max_sdf_instances,
            instance_data_floats,
            effects_data_floats,
            sound_data_floats,
            event_data_floats,
            sdf_data_floats,
            instance_data_offset,
            effects_data_offset,
            sound_data_offset,
            event_data_offset,
            sdf_data_offset,
            buffer_total_floats,
            buffer_total_bytes,
        }
    }

    /// Compute layout from a GameConfig.
    pub fn from_config(config: &GameConfig) -> Self {
        Self::new(
            config.max_instances,
            config.max_effects_vertices,
            config.max_sounds,
            config.max_events,
            config.max_sdf_instances,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Old hardcoded values for regression testing.
    const OLD_MAX_INSTANCES: usize = 512;
    const OLD_MAX_EFFECTS_VERTICES: usize = 16384;
    const OLD_MAX_SOUNDS: usize = 32;
    const OLD_MAX_EVENTS: usize = 32;
    const DEFAULT_MAX_SDF_INSTANCES: usize = 128;
    const OLD_INSTANCE_DATA_FLOATS: usize = OLD_MAX_INSTANCES * 8;
    const OLD_EFFECTS_DATA_FLOATS: usize = OLD_MAX_EFFECTS_VERTICES * 5;
    const OLD_SOUND_DATA_FLOATS: usize = OLD_MAX_SOUNDS;
    const OLD_EVENT_DATA_FLOATS: usize = OLD_MAX_EVENTS * 4;
    const DEFAULT_SDF_DATA_FLOATS: usize = DEFAULT_MAX_SDF_INSTANCES * 12;
    const NEW_HEADER_FLOATS: usize = 16;
    const OLD_INSTANCE_DATA_OFFSET: usize = NEW_HEADER_FLOATS;
    const OLD_EFFECTS_DATA_OFFSET: usize = OLD_INSTANCE_DATA_OFFSET + OLD_INSTANCE_DATA_FLOATS;
    const OLD_SOUND_DATA_OFFSET: usize = OLD_EFFECTS_DATA_OFFSET + OLD_EFFECTS_DATA_FLOATS;
    const OLD_EVENT_DATA_OFFSET: usize = OLD_SOUND_DATA_OFFSET + OLD_SOUND_DATA_FLOATS;
    const OLD_SDF_DATA_OFFSET: usize = OLD_EVENT_DATA_OFFSET + OLD_EVENT_DATA_FLOATS;
    const OLD_BUFFER_TOTAL_FLOATS: usize = OLD_SDF_DATA_OFFSET + DEFAULT_SDF_DATA_FLOATS;

    #[test]
    fn from_default_config_matches_expected_sizes() {
        let layout = ProtocolLayout::from_config(&GameConfig::default());

        assert_eq!(layout.max_instances, OLD_MAX_INSTANCES);
        assert_eq!(layout.max_effects_vertices, OLD_MAX_EFFECTS_VERTICES);
        assert_eq!(layout.max_sounds, OLD_MAX_SOUNDS);
        assert_eq!(layout.max_events, OLD_MAX_EVENTS);
        assert_eq!(layout.max_sdf_instances, DEFAULT_MAX_SDF_INSTANCES);

        assert_eq!(layout.instance_data_floats, OLD_INSTANCE_DATA_FLOATS);
        assert_eq!(layout.effects_data_floats, OLD_EFFECTS_DATA_FLOATS);
        assert_eq!(layout.sound_data_floats, OLD_SOUND_DATA_FLOATS);
        assert_eq!(layout.event_data_floats, OLD_EVENT_DATA_FLOATS);
        assert_eq!(layout.sdf_data_floats, DEFAULT_SDF_DATA_FLOATS);

        assert_eq!(layout.instance_data_offset, OLD_INSTANCE_DATA_OFFSET);
        assert_eq!(layout.effects_data_offset, OLD_EFFECTS_DATA_OFFSET);
        assert_eq!(layout.sound_data_offset, OLD_SOUND_DATA_OFFSET);
        assert_eq!(layout.event_data_offset, OLD_EVENT_DATA_OFFSET);
        assert_eq!(layout.sdf_data_offset, OLD_SDF_DATA_OFFSET);

        assert_eq!(layout.buffer_total_floats, OLD_BUFFER_TOTAL_FLOATS);
        assert_eq!(layout.buffer_total_bytes, OLD_BUFFER_TOTAL_FLOATS * 4);
    }

    #[test]
    fn custom_capacities_compute_correctly() {
        let layout = ProtocolLayout::new(256, 8192, 16, 64, 64);

        assert_eq!(layout.instance_data_floats, 256 * 8);
        assert_eq!(layout.effects_data_floats, 8192 * 5);
        assert_eq!(layout.sound_data_floats, 16);
        assert_eq!(layout.event_data_floats, 64 * 4);
        assert_eq!(layout.sdf_data_floats, 64 * 12);

        let expected_total = HEADER_FLOATS
            + 256 * 8
            + 8192 * 5
            + 16
            + 64 * 4
            + 64 * 12;
        assert_eq!(layout.buffer_total_floats, expected_total);
        assert_eq!(layout.buffer_total_bytes, expected_total * 4);
    }

    #[test]
    fn offsets_are_contiguous() {
        let layout = ProtocolLayout::new(100, 200, 10, 20, 50);

        assert_eq!(layout.instance_data_offset, HEADER_FLOATS);
        assert_eq!(layout.effects_data_offset, layout.instance_data_offset + layout.instance_data_floats);
        assert_eq!(layout.sound_data_offset, layout.effects_data_offset + layout.effects_data_floats);
        assert_eq!(layout.event_data_offset, layout.sound_data_offset + layout.sound_data_floats);
        assert_eq!(layout.sdf_data_offset, layout.event_data_offset + layout.event_data_floats);
        assert_eq!(layout.buffer_total_floats, layout.sdf_data_offset + layout.sdf_data_floats);
    }

    #[test]
    fn existing_offsets_unchanged_with_sdf() {
        // Verify adding SDF doesn't change the offsets of existing sections
        let layout = ProtocolLayout::new(512, 16384, 32, 32, 128);
        assert_eq!(layout.instance_data_offset, 16);
        assert_eq!(layout.effects_data_offset, 16 + 512 * 8);
        assert_eq!(layout.sound_data_offset, 16 + 512 * 8 + 16384 * 5);
        assert_eq!(layout.event_data_offset, 16 + 512 * 8 + 16384 * 5 + 32);
    }
}
