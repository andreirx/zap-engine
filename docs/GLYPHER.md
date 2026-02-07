# Vision: GLYPHER

## 1. The Brainstorm

“we are going to have a canvas for writing one letter on 80% of the screen and the _ _ _ _ filling up with letters as you write them on the top 20% to show you the word you are currently writing (and for a nice touch the "completed" letters in HDR will be moving to the left and out of the drawing canvas but they will stay on the top 20% part. and the current letter strokes will be shown front and center and you have to follow them to "get" the current letter and move to the next. and word by word we will reveal entire sentences - I will put a list of proverbs or something like that the game will pick from. and I will define in an editor separately each letter strokes and will need to handle modifiers - like when you write a lowercase o, v, w or b your final stroke is above the baseline so the next letter has to pick up from there so we need to handle all this.. so this is the spec. on the other hand I want to package zap-engine before moving on - what do we need to do? make a npm package? a cargo package? I don't know how things work in this area - explain and tell me what I need to do”

---

## 2. The spec

Your requirement for `o`, `v`, `w` changing the next letter's start point is technically called **Contextual Alternates** in typography.

We need a data structure that handles "Entry" and "Exit" points.

### The Data Model (Rust)

Instead of a simple `Vec<Vec2>`, a Letter definition becomes a state machine.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StrokeConnection {
    Baseline, // Standard start (bottom)
    High,     // High start (top, after o, v, w, b)
}

pub struct GlyphDefinition {
    pub char: char,
    // Variants: How to draw 'a' depending on where we came from
    pub variants: HashMap<StrokeConnection, Vec<GlyphPath>>,
    // Where this letter ends (to tell the NEXT letter where to start)
    pub exit_type: StrokeConnection, 
}

// Example Data for 'o'
// exit_type: StrokeConnection::High

// Example Data for 'a'
// variants: {
//    Baseline: [ ... standard 'a' strokes ... ],
//    High:     [ ... special 'a' that starts with a downward bridge ... ]
// }

```

### The Visual Layout (80/20 Split)

This is purely a **Camera/Viewport** setup in `zap-engine`.

* **The World:** The infinite scroll of text exists at `Y = 0` to `Y = 1000`.
* **The Camera:** * The "Completed Words" (Top 20%) are standard Sprites/Vectors placed at world coordinates `Y = 800`.
* The "Drawing Area" (Bottom 80%) is where the camera is focused `Y = 400`.


* **The Flow:**
1. User traces a letter in the center.
2. On success, the vector strokes "morph" (lerp) into the official font vector.
3. The camera stays still, but the *completed letter* animates (interpolates) upwards to `Y = 800` and joins the sentence.
4. The sentence slides left.



---

## 3. The Editor (Immediate Necessity)

You cannot define these strokes by guessing coordinates. You need a tool.

**Action:** Build a simple "Glyph Editor" scene inside the ZapEngine repo (`examples/editor`).

**Editor Features:**

1. **Background:** Loads the font image (your reference) or a standard font character.
2. **Input:** You click to add points.
3. **Mode Switch:** "Defining Baseline Variant" vs "Defining High Variant".
4. **Export:** A button that dumps the JSON to the console.

```json
// output.json
"a": {
  "exit": "Baseline",
  "variants": {
    "Baseline": [ [[0,0], [10,20], ... ] ],
    "High": [ [[0, 50], [10, 20], ... ] ]
  }
}

```
