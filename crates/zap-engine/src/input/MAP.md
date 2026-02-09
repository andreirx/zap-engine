# input/

Input event handling for pointer, keyboard, resize, and custom events.

## Files

| File | Purpose |
|------|---------|
| `queue.rs` | `InputQueue`, `InputEvent` enum |

## Key Types

- **`InputQueue`**: Ring buffer of input events with `push()` and `drain()`
- **`InputEvent`**: Enum with variants:
  - `PointerDown/Up/Move { pos: Vec2 }` — world-space coordinates
  - `KeyDown/KeyUp { key: String }`
  - `Resize { width, height }`
  - `Custom { kind: u32, a: f32, b: f32, c: f32 }` — React→Rust events

## Data Flow

```
Browser events (CSS px)
    → Worker screenToWorld()
    → world coordinates
    → game_pointer_*/game_key_*/game_custom_event
    → InputQueue.push()
```

## Architecture Notes

All pointer events arrive in world coordinates after the worker transforms them. This allows games to ignore screen resolution and work purely in world units.

Custom events enable React UI to communicate with Rust game logic (e.g., "Reset" button, element selector).
