/// Input event types the engine understands.
/// Generic — no game-specific semantics.
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    /// A touch/click began at world coordinates (x, y).
    PointerDown { x: f32, y: f32 },
    /// A touch/click ended at world coordinates (x, y).
    PointerUp { x: f32, y: f32 },
    /// A touch/cursor moved to world coordinates (x, y).
    PointerMove { x: f32, y: f32 },
    /// A key was pressed.
    KeyDown { key_code: u32 },
    /// A key was released.
    KeyUp { key_code: u32 },
    /// A custom event from the UI layer (React buttons, etc.).
    /// `kind` identifies the event type; `a`, `b`, `c` carry arbitrary data.
    Custom { kind: u32, a: f32, b: f32, c: f32 },
    /// The wheel turned over world coordinates (x, y). `dx`, `dy` are the
    /// browser's `WheelEvent.deltaX` / `deltaY`, unscaled (unit set by its `deltaMode`).
    Wheel { dx: f32, dy: f32, x: f32, y: f32 },
}

/// A queue of input events.
/// JS writes events into the queue; Rust reads and drains them each frame.
pub struct InputQueue {
    events: Vec<InputEvent>,
}

impl InputQueue {
    pub fn new() -> Self {
        Self {
            events: Vec::with_capacity(32),
        }
    }

    /// Push a new input event (called from JS via wasm-bindgen).
    pub fn push(&mut self, event: InputEvent) {
        self.events.push(event);
    }

    /// Drain all pending events. Returns a Vec and clears the queue.
    pub fn drain(&mut self) -> Vec<InputEvent> {
        std::mem::take(&mut self.events)
    }

    /// Iterate over pending events without consuming them.
    pub fn iter(&self) -> impl Iterator<Item = &InputEvent> {
        self.events.iter()
    }

    /// Check if there are pending events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Number of pending events.
    pub fn len(&self) -> usize {
        self.events.len()
    }
}

impl Default for InputQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_drain() {
        let mut q = InputQueue::new();
        q.push(InputEvent::PointerDown { x: 10.0, y: 20.0 });
        q.push(InputEvent::KeyDown { key_code: 32 });
        assert_eq!(q.len(), 2);
        let events = q.drain();
        assert_eq!(events.len(), 2);
        assert!(q.is_empty());
    }

    #[test]
    fn custom_event() {
        let mut q = InputQueue::new();
        q.push(InputEvent::Custom { kind: 7, a: 1.5, b: 2.5, c: 3.5 });
        let events = q.drain();
        assert_eq!(events.len(), 1);
        match events[0] {
            InputEvent::Custom { kind, a, b, c } => {
                assert_eq!(kind, 7);
                assert_eq!(a, 1.5);
                assert_eq!(b, 2.5);
                assert_eq!(c, 3.5);
            }
            _ => panic!("Expected Custom event"),
        }
    }

    #[test]
    fn wheel_keeps_its_place_among_pointer_events() {
        let mut q = InputQueue::new();
        q.push(InputEvent::PointerDown { x: 1.0, y: 2.0 });
        q.push(InputEvent::Wheel { dx: 0.0, dy: 100.0, x: 400.0, y: 300.0 });
        q.push(InputEvent::PointerUp { x: 3.0, y: 4.0 });
        let events = q.drain();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], InputEvent::PointerDown { x, y } if x == 1.0 && y == 2.0));
        assert!(matches!(
            events[1],
            InputEvent::Wheel { dx, dy, x, y } if dx == 0.0 && dy == 100.0 && x == 400.0 && y == 300.0
        ));
        assert!(matches!(events[2], InputEvent::PointerUp { x, y } if x == 3.0 && y == 4.0));
    }
}
