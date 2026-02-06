/// Render layer — controls draw order for entities.
///
/// Layers are drawn back-to-front: Background first, UI last.
/// Within a layer, entities are grouped by atlas for batched rendering.
/// Default layer is `Objects` — existing games work unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum RenderLayer {
    Background = 0,
    Terrain = 1,
    #[default]
    Objects = 2,
    Foreground = 3,
    VFX = 4,
    UI = 5,
}

impl RenderLayer {
    /// Total number of render layers.
    pub const COUNT: usize = 6;

    /// Convert from a u8 value to a RenderLayer.
    /// Returns None if the value is out of range.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Background),
            1 => Some(Self::Terrain),
            2 => Some(Self::Objects),
            3 => Some(Self::Foreground),
            4 => Some(Self::VFX),
            5 => Some(Self::UI),
            _ => None,
        }
    }

    /// Convert to u8 for protocol serialization.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_objects() {
        assert_eq!(RenderLayer::default(), RenderLayer::Objects);
    }

    #[test]
    fn ordering_is_back_to_front() {
        assert!(RenderLayer::Background < RenderLayer::Terrain);
        assert!(RenderLayer::Terrain < RenderLayer::Objects);
        assert!(RenderLayer::Objects < RenderLayer::Foreground);
        assert!(RenderLayer::Foreground < RenderLayer::VFX);
        assert!(RenderLayer::VFX < RenderLayer::UI);
    }

    #[test]
    fn round_trip_u8() {
        for val in 0..RenderLayer::COUNT as u8 {
            let layer = RenderLayer::from_u8(val).unwrap();
            assert_eq!(layer.as_u8(), val);
        }
        assert!(RenderLayer::from_u8(6).is_none());
    }

    #[test]
    fn count_is_correct() {
        assert_eq!(RenderLayer::COUNT, 6);
    }
}
