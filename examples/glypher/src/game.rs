use glam::Vec2;
use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::input::queue::InputQueue;

use crate::glyphs::BakedGlyphs;
use crate::sayings::SayingsDB;
use crate::tracing::{StrokeTracer, TraceResult, TraceState};

// --- World layout ---
const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;

/// Top zone height for word hints.
const TOP_ZONE_H: f32 = 100.0;

/// Base glyph box dimensions at scale 1.0 for the main traced glyph.
/// Width stays fixed; height adapts to visible area on portrait viewports.
const GLYPH_BASE_W: f32 = 600.0;
const GLYPH_BASE_H: f32 = 480.0;

/// Tracing threshold in world units (~finger width on touchscreen).
const TRACE_THRESHOLD: f32 = 35.0;

/// Guide light speed (path points per second).
const GUIDE_SPEED: f32 = 60.0;

/// Duration of letter celebration (seconds).
const CELEBRATE_DURATION: f32 = 1.2;
/// Duration of saying-complete celebration (seconds).
const SAYING_CELEBRATE_DURATION: f32 = 12.0;
/// Duration of transition to next letter (seconds).
const TRANSITION_DURATION: f32 = 0.4;
/// Duration of failed stroke red flash (seconds).
const FAIL_FADE_TIME: f32 = 0.5;

/// Size of miniature glyphs in top zone.
const HINT_GLYPH_SCALE: f32 = 0.12;
/// Hint zone row height.
const HINT_ROW_H: f32 = 30.0;
/// Top margin — must be >= half hint glyph height so glyphs don't clip above visible area.
/// GLYPH_BASE_H * HINT_GLYPH_SCALE / 2 ≈ 29, plus padding.
const HINT_TOP_MARGIN: f32 = 35.0;

/// Word gap as a fraction of a standard character advance.
const WORD_GAP_FRACTION: f32 = 0.5;

/// Letters that should use High variant at the start of a word.
const WORD_START_HIGH: &[char] = &['m', 'n', 'v', 'w'];

// --- Game event kinds (Rust → React) ---
const EVENT_SAYING_COMPLETE: f32 = 1.0;
const EVENT_LETTER_COMPLETE: f32 = 2.0;

// --- Custom event kinds (React → Rust) ---
const _CUSTOM_RESTART: u32 = 1;
/// Worker sends visible world dimensions on viewport resize.
const CUSTOM_VIEWPORT: u32 = 99;

// --- Embedded data ---
const GLYPHS_JSON: &str = include_str!("../data/glyphs_baked.json");
const SAYINGS_JSON: &str = include_str!("../data/sayings.json");

/// Simple xorshift64 RNG (avoids depending on engine internals).
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 1 } else { seed } }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x & 0xFFFF_FFFF) as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GamePhase {
    PickSaying,
    ShowLetter,
    Tracing,
    LetterCelebration,
    TransitionNext,
    SayingComplete,
}

/// A completed stroke stored for rendering.
struct CompletedStroke {
    points: Vec<Vec2>,
}

/// Info about a completed letter for the top-zone display.
struct CompletedLetter {
    _ch: char,
    variant: String,
    word_idx: usize,
    letter_idx: usize,
}

pub struct Glypher {
    glyphs: BakedGlyphs,
    sayings: SayingsDB,
    rng: Rng,

    // Current saying state
    current_saying: String,
    words: Vec<String>,
    word_idx: usize,
    letter_idx: usize,
    stroke_idx: usize,
    current_variant: String,
    prev_exit: String,

    // Stroke tracing
    tracer: StrokeTracer,
    completed_strokes: Vec<CompletedStroke>,
    failed_stroke: Option<(Vec<Vec2>, f32)>,

    // Current letter's strokes in world coords (cached)
    current_strokes_world: Vec<Vec<Vec2>>,

    // Guide light
    guide_time: f32,

    // Phase timers
    celebrate_timer: f32,
    transition_timer: f32,
    show_timer: f32,

    // Completed letters for top zone
    completed_letters: Vec<CompletedLetter>,

    phase: GamePhase,

    // Background entity spawned flag
    bg_spawned: bool,

    // Viewport-adaptive layout: visible world dimensions from projection.
    // Updated via CUSTOM_VIEWPORT event from the worker on resize.
    visible_w: f32,
    visible_h: f32,
}

impl Glypher {
    pub fn new() -> Self {
        Self {
            glyphs: BakedGlyphs::from_json("{}").unwrap_or_else(|_| {
                BakedGlyphs::from_json(r#"{"meta":{"highExitLetters":[],"widths":{}},"glyphs":{}}"#).unwrap()
            }),
            sayings: SayingsDB::from_json("[]").unwrap(),
            rng: Rng::new(12345),

            current_saying: String::new(),
            words: Vec::new(),
            word_idx: 0,
            letter_idx: 0,
            stroke_idx: 0,
            current_variant: "Baseline".to_string(),
            prev_exit: "Baseline".to_string(),

            tracer: StrokeTracer::new(Vec::new(), TRACE_THRESHOLD),
            completed_strokes: Vec::new(),
            failed_stroke: None,
            current_strokes_world: Vec::new(),

            guide_time: 0.0,

            celebrate_timer: 0.0,
            transition_timer: 0.0,
            show_timer: 0.0,

            completed_letters: Vec::new(),

            phase: GamePhase::PickSaying,
            bg_spawned: false,
            visible_w: WORLD_W,
            visible_h: WORLD_H,
        }
    }

    // --- Viewport-adaptive layout helpers ---
    // Camera projection: (0,0) = screen top-left, always.
    // On portrait viewports, extra visible space extends DOWNWARD (visible_h > WORLD_H).
    // Hint text lives near y=0 (always at screen top).
    // The glyph drawing area fills from TOP_ZONE_H down to visible_h.

    /// Start of the glyph drawing zone (below the hint zone).
    fn draw_zone_top(&self) -> f32 {
        TOP_ZONE_H
    }

    /// Height of the glyph drawing zone — grows on portrait viewports.
    fn draw_zone_h(&self) -> f32 {
        self.visible_h - TOP_ZONE_H
    }

    /// Uniform scale factor for the main traced glyph.
    /// On portrait, the glyph grows proportionally (both width AND height)
    /// so it fills more of the screen without distorting the aspect ratio.
    /// Width is constrained by the widest letter's STROKE extent (not box extent),
    /// allowing the glyph box to extend off-screen while strokes stay visible.
    fn glyph_scale(&self) -> f32 {
        let max_h = self.draw_zone_h() * 0.85;
        let scale_h = max_h / GLYPH_BASE_H;

        // Constrain width so widest letters' strokes fit within visible area.
        // Width-2 (m, w): box = 1.3 × BASE_W, strokes span 62% of box.
        let max_stroke_w = self.visible_w * 0.95;
        let widest_stroke_base = GLYPH_BASE_W * 1.3 * Self::glyph_span(2);
        let scale_w = max_stroke_w / widest_stroke_base;

        scale_w.min(scale_h).max(1.0)
    }

    /// Effective glyph height — scales proportionally with glyph_scale().
    fn glyph_h(&self) -> f32 {
        GLYPH_BASE_H * self.glyph_scale()
    }

    /// Vertical origin of the glyph drawing area (top-left y).
    fn glyph_origin_y(&self) -> f32 {
        self.draw_zone_top() + (self.draw_zone_h() - self.glyph_h()) / 2.0
    }

    /// Center of the glyph drawing area.
    fn glyph_center(&self) -> Vec2 {
        Vec2::new(WORLD_W / 2.0, self.glyph_origin_y() + self.glyph_h() / 2.0)
    }

    /// Glyph box width for a given width class.
    fn glyph_box_width(ch_width: u8) -> f32 {
        match ch_width {
            0 => GLYPH_BASE_W * 0.5,
            2 => GLYPH_BASE_W * 1.3,
            _ => GLYPH_BASE_W,
        }
    }

    /// Convert glyph normalized coordinates [0,1] to world coordinates,
    /// taking into account the letter's width class and current viewport.
    fn glyph_to_world(&self, gx: f32, gy: f32, width: u8) -> Vec2 {
        let scale = self.glyph_scale();
        let w = Self::glyph_box_width(width) * scale;
        let origin_x = (WORLD_W - w) / 2.0;
        Vec2::new(
            gx * w + origin_x,
            gy * self.glyph_h() + self.glyph_origin_y(),
        )
    }

    /// Convert a stroke (list of [x,y] normalized points) to world coordinates.
    fn stroke_to_world(&self, stroke: &[[f32; 2]], width: u8) -> Vec<Vec2> {
        stroke.iter()
            .map(|p| self.glyph_to_world(p[0], p[1], width))
            .collect()
    }

    /// Get the current character being traced.
    fn current_char(&self) -> Option<char> {
        let word = self.words.get(self.word_idx)?;
        word.chars().nth(self.letter_idx)
    }

    /// Pick a new saying randomly and reset all letter state.
    fn pick_saying(&mut self) {
        let idx = self.rng.next_u32() as usize;
        self.current_saying = self.sayings.pick(idx).to_string();
        self.words = self.current_saying.split_whitespace().map(String::from).collect();
        self.word_idx = 0;
        self.letter_idx = 0;
        self.stroke_idx = 0;
        self.completed_strokes.clear();
        self.completed_letters.clear();
        self.failed_stroke = None;
        self.prev_exit = "Baseline".to_string();
        self.setup_current_letter();
    }

    /// Determine the variant for a character, including word-start-High rule.
    fn effective_variant(&self, ch: char) -> &'static str {
        // At word start (letter_idx == 0), m/n/v/w use High
        if self.letter_idx == 0 && WORD_START_HIGH.contains(&ch) {
            return "High";
        }
        self.glyphs.variant_for(ch, &self.prev_exit)
    }

    /// Set up tracing state for the current letter.
    fn setup_current_letter(&mut self) {
        let Some(ch) = self.current_char() else { return; };

        // Determine variant based on previous letter's exit type + word-start rules
        self.current_variant = self.effective_variant(ch).to_string();

        // Get all strokes for this letter and convert to world coords
        let width = self.glyphs.width(ch);
        if let Some(strokes) = self.glyphs.get_strokes(ch, &self.current_variant) {
            self.current_strokes_world = strokes.iter()
                .map(|s| self.stroke_to_world(s, width))
                .collect();
        } else {
            // Fallback: try the other variant or Default
            self.current_strokes_world = Vec::new();
            log::warn!("No strokes for '{}' variant '{}'", ch, self.current_variant);
        }

        self.stroke_idx = 0;
        self.completed_strokes.clear();
        self.failed_stroke = None;
        self.guide_time = 0.0;

        // Set up tracer for the first stroke
        self.setup_current_stroke();
    }

    /// Set up the tracer for the current stroke index.
    fn setup_current_stroke(&mut self) {
        if let Some(stroke_world) = self.current_strokes_world.get(self.stroke_idx) {
            // Scale threshold with glyph size so touch targets stay proportional
            let threshold = TRACE_THRESHOLD * self.glyph_scale();
            self.tracer = StrokeTracer::new(stroke_world.clone(), threshold);
            self.guide_time = 0.0;
        }
    }

    /// Called when the visible area changes (viewport resize / orientation change).
    /// Re-computes current letter's world coordinates for the new layout.
    fn on_viewport_change(&mut self) {
        if self.current_strokes_world.is_empty() {
            return;
        }
        let Some(ch) = self.current_char() else { return; };
        let width = self.glyphs.width(ch);
        if let Some(strokes) = self.glyphs.get_strokes(ch, &self.current_variant) {
            self.current_strokes_world = strokes.iter()
                .map(|s| self.stroke_to_world(s, width))
                .collect();
            // Re-setup the current stroke tracer with new world coords
            self.setup_current_stroke();
        }
    }

    /// Advance to the next stroke, or complete the letter.
    fn advance_stroke(&mut self) -> bool {
        // Save completed stroke
        let points = std::mem::take(&mut self.tracer.user_points);
        self.completed_strokes.push(CompletedStroke { points });

        self.stroke_idx += 1;
        if self.stroke_idx < self.current_strokes_world.len() {
            // More strokes to trace
            self.setup_current_stroke();
            false
        } else {
            // All strokes complete
            true
        }
    }

    /// Advance to the next letter, or complete the word.
    fn advance_letter(&mut self) {
        let ch = self.current_char().unwrap_or(' ');
        let variant = self.current_variant.clone();

        // Record completion
        self.completed_letters.push(CompletedLetter {
            _ch: ch,
            variant,
            word_idx: self.word_idx,
            letter_idx: self.letter_idx,
        });

        // Update exit type for contextual alternates
        self.prev_exit = self.glyphs.exit_type(ch).to_string();

        // Move to next letter
        self.letter_idx += 1;
        let word = &self.words[self.word_idx];
        if self.letter_idx >= word.len() {
            // Word complete — advance to next word
            self.word_idx += 1;
            self.letter_idx = 0;
            // After a space, reset to Baseline entry
            self.prev_exit = "Baseline".to_string();

            if self.word_idx >= self.words.len() {
                // Saying complete!
                self.phase = GamePhase::SayingComplete;
                self.celebrate_timer = SAYING_CELEBRATE_DURATION;
                return;
            }
        }

        self.setup_current_letter();
    }

    /// Spawn the background entity (once, deferred until manifest is loaded).
    /// NOTE: The renderer uses only `scale.x` and renders a SQUARE sprite.
    /// Size must be large enough to cover extreme portrait viewports.
    fn ensure_background(&mut self, ctx: &mut EngineContext) {
        if self.bg_spawned {
            return;
        }

        // Only set bg_spawned after successful spawn — manifest loads async after init()
        if let Some(bg_sprite) = ctx.sprite("bg") {
            let bg_id = ctx.next_id();
            // Sprite is always a SQUARE (shader uses scale.x for both dimensions).
            // 3× WORLD_W covers up to 3:1 portrait aspect.
            let bg_size = WORLD_W * 3.0;
            ctx.scene.spawn(
                Entity::new(bg_id)
                    .with_tag("bg")
                    .with_pos(Vec2::new(WORLD_W / 2.0, bg_size / 2.0))
                    .with_scale(Vec2::new(bg_size, bg_size))
                    .with_layer(RenderLayer::Background)
                    .with_sprite(bg_sprite),
            );
            self.bg_spawned = true;
        }
    }

    /// Draw guide strokes (faint blue) for the current letter.
    fn draw_guide_strokes(&self, ctx: &mut EngineContext) {
        let guide_color = VectorColor::new(0.4, 0.6, 1.2, 0.4);
        for (i, stroke) in self.current_strokes_world.iter().enumerate() {
            if stroke.len() < 2 {
                continue;
            }
            // Already-completed strokes are dimmer
            let color = if i < self.stroke_idx {
                VectorColor::new(0.2, 0.4, 0.6, 0.15)
            } else {
                guide_color
            };
            ctx.vectors.stroke_polyline(stroke, 6.0, color);
        }
    }

    /// Draw the moving guide light and dot at the current animated position.
    /// guide_time is advanced in the state machine; this only reads it.
    fn draw_guide_light(&self, ctx: &mut EngineContext) {
        if let Some(stroke) = self.current_strokes_world.get(self.stroke_idx) {
            if stroke.is_empty() {
                return;
            }

            let total = stroke.len() as f32;
            let cycle = total + 40.0;
            let t = self.guide_time % cycle;
            let idx = (t as usize).min(stroke.len() - 1);
            let pos = stroke[idx];

            // The MOVING guide light — sweeps across the bump-mapped background
            ctx.lights.add(
                PointLight::new(pos, [0.5, 0.7, 1.0], 3.0, 280.0)
            );

            // Glow dot at guide position
            ctx.vectors.fill_circle(pos, 8.0, VectorColor::new(0.8, 1.2, 2.5, 0.9));
        }
    }

    /// A faint static light centered on the whole letter area (context glow).
    fn draw_letter_ambient_light(&self, ctx: &mut EngineContext) {
        let center = self.glyph_center();
        ctx.lights.add(
            PointLight::new(center, [0.3, 0.3, 0.4], 4.0, 350.0)
        );
    }

    /// A single green light that follows the user's cursor while drawing.
    fn draw_user_cursor_light(&self, ctx: &mut EngineContext) {
        if self.tracer.state == TraceState::Tracing {
            if let Some(&pos) = self.tracer.user_points.last() {
                ctx.lights.add(
                    PointLight::new(pos, [0.3, 1.0, 0.4], 4.0, 250.0)
                );
            }
        }
    }

    /// Draw completed strokes (green HDR).
    fn draw_completed_strokes(&self, ctx: &mut EngineContext) {
        let green = VectorColor::new(0.2, 2.5, 0.5, 1.0);
        for stroke in &self.completed_strokes {
            if stroke.points.len() >= 2 {
                ctx.vectors.stroke_polyline(&stroke.points, 8.0, green);
            }
        }
    }

    /// Draw the user's in-progress stroke.
    fn draw_user_stroke(&self, ctx: &mut EngineContext) {
        if self.tracer.state == TraceState::Tracing && self.tracer.user_points.len() >= 2 {
            let green = VectorColor::new(0.2, 2.5, 0.5, 1.0);
            ctx.vectors.stroke_polyline(&self.tracer.user_points, 8.0, green);
        }
    }

    /// Draw failed stroke (red, fading).
    fn draw_failed_stroke(&self, ctx: &mut EngineContext) {
        if let Some((ref points, timer)) = self.failed_stroke {
            if points.len() >= 2 && timer > 0.0 {
                let alpha = (timer / FAIL_FADE_TIME).min(1.0);
                let red = VectorColor::new(2.5, 0.2, 0.1, alpha);
                ctx.vectors.stroke_polyline(points, 6.0, red);
            }
        }
    }

    /// A fading red light when stroke fails.
    fn draw_fail_light(&self, ctx: &mut EngineContext) {
        if let Some((_, timer)) = self.failed_stroke {
            if timer > 0.0 {
                let t = (timer / FAIL_FADE_TIME).min(1.0);
                let center = self.glyph_center();
                ctx.lights.add(
                    PointLight::new(center, [1.0, 0.1, 0.05], 12.0 * t, 350.0)
                );
            }
        }
    }

    /// Draw letter celebration effects — bright golden light floods the letter.
    fn draw_letter_celebration(&self, ctx: &mut EngineContext) {
        let t = (self.celebrate_timer / CELEBRATE_DURATION).clamp(0.0, 1.0);

        let gold = VectorColor::new(8.0 * t, 6.8 * t, 2.4 * t, 1.0);
        let stroke_width = 8.0 + 12.0 * t;

        // Draw all strokes as golden, wide, glowing
        for stroke in &self.current_strokes_world {
            if stroke.len() >= 2 {
                ctx.vectors.stroke_polyline(stroke, stroke_width, gold);
            }
        }

        // Golden light at glyph center
        let center = self.glyph_center();
        ctx.lights.add(
            PointLight::new(center, [1.0, 0.85, 0.3], 14.0 * t, 350.0)
        );
    }

    /// Draw saying-complete golden light (no glyph strokes — just the light).
    fn draw_saying_celebration_light(&self, ctx: &mut EngineContext) {
        let t = (self.celebrate_timer / SAYING_CELEBRATE_DURATION).clamp(0.0, 1.0);
        let center = Vec2::new(WORLD_W / 2.0, self.visible_h / 2.0);
        ctx.lights.add(
            PointLight::new(center, [1.0, 0.85, 0.3], 10.0 * t, 400.0)
        );
    }

    // --- Cursive spacing: advance = glyph_box_width × glyph_span ---
    // Per-width-class span derived from baked glyph data (entry→exit x extent):
    //   Width 0 (narrow): [0.41, 0.60] → span ≈ 0.19
    //   Width 1 (standard): [0.30, 0.70] → span ≈ 0.40
    //   Width 2 (wide):     [0.195, 0.81] → span ≈ 0.62

    /// Fraction of the [0,1] box that strokes actually span, per width class.
    fn glyph_span(ch_width: u8) -> f32 {
        match ch_width {
            0 => 0.19,
            2 => 0.62,
            _ => 0.40,
        }
    }

    /// Character advance width at a given render scale.
    fn char_advance_at_scale(&self, ch: char, scale: f32) -> f32 {
        let w = self.glyphs.width(ch);
        Self::glyph_box_width(w) * scale * Self::glyph_span(w)
    }

    /// Word width at a given scale (sum of char advances).
    fn word_width_at_scale(&self, word: &str, scale: f32) -> f32 {
        word.chars().map(|ch| self.char_advance_at_scale(ch, scale)).sum()
    }

    /// Word gap at a given scale — fraction of a standard character advance.
    fn word_gap_at_scale(&self, scale: f32) -> f32 {
        GLYPH_BASE_W * scale * Self::glyph_span(1) * WORD_GAP_FRACTION
    }

    /// Draw a glyph at an arbitrary position and scale.
    fn draw_glyph_at_scale(
        &self, ctx: &mut EngineContext, ch: char, variant: &str,
        center_x: f32, center_y: f32, scale: f32,
        color: VectorColor, stroke_w: f32,
    ) {
        let width = self.glyphs.width(ch);
        if let Some(strokes) = self.glyphs.get_strokes(ch, variant) {
            let w_scale = match width {
                0 => GLYPH_BASE_W * 0.5,
                2 => GLYPH_BASE_W * 1.3,
                _ => GLYPH_BASE_W,
            } * scale;
            let h_scale = GLYPH_BASE_H * scale;

            for stroke in strokes {
                let world_pts: Vec<Vec2> = stroke.iter()
                    .map(|p| Vec2::new(
                        center_x + (p[0] - 0.5) * w_scale,
                        center_y + (p[1] - 0.5) * h_scale,
                    ))
                    .collect();
                if world_pts.len() >= 2 {
                    ctx.vectors.stroke_polyline(&world_pts, stroke_w, color);
                }
            }
        }
    }

    /// Word-wrap words into lines that fit within `max_width` at the given scale.
    /// Returns Vec of (line_word_indices, line_width).
    fn wrap_words_at_scale(&self, scale: f32, max_width: f32) -> Vec<(Vec<usize>, f32)> {
        let gap = self.word_gap_at_scale(scale);
        let mut lines: Vec<(Vec<usize>, f32)> = vec![(vec![], 0.0)];

        for (wi, word) in self.words.iter().enumerate() {
            let ww = self.word_width_at_scale(word, scale);
            let (ref indices, current_w) = lines.last().unwrap();
            let needed = if indices.is_empty() { ww } else { gap + ww };

            if current_w + needed > max_width && !indices.is_empty() {
                // Start new line
                lines.push((vec![wi], ww));
            } else {
                let new_w = current_w + needed;
                lines.last_mut().unwrap().0.push(wi);
                lines.last_mut().unwrap().1 = new_w;
            }
        }
        lines
    }

    /// Draw word hints at the top of the visible screen (with wrapping).
    fn draw_word_hints(&self, ctx: &mut EngineContext) {
        let scale = HINT_GLYPH_SCALE;
        let max_width = WORLD_W * 0.95;
        let lines = self.wrap_words_at_scale(scale, max_width);
        let gap = self.word_gap_at_scale(scale);
        let hint_top = HINT_TOP_MARGIN;

        for (row, (word_indices, line_w)) in lines.iter().enumerate() {
            let y = hint_top + HINT_ROW_H * 0.5 + row as f32 * HINT_ROW_H;
            if y > hint_top + TOP_ZONE_H { break; } // Don't draw below hint zone

            let mut cursor_x = (WORLD_W - line_w) / 2.0;

            for &wi in word_indices {
                let word = &self.words[wi];
                for (li, ch) in word.chars().enumerate() {
                    let advance = self.char_advance_at_scale(ch, scale);
                    let center_x = cursor_x + advance / 2.0;

                    let is_completed = self.completed_letters.iter().any(|cl| {
                        cl.word_idx == wi && cl.letter_idx == li
                    });
                    let is_current = wi == self.word_idx && li == self.letter_idx;
                    let is_past_word = wi < self.word_idx;

                    if is_completed || is_past_word {
                        let variant = self.completed_letters.iter()
                            .find(|cl| cl.word_idx == wi && cl.letter_idx == li)
                            .map(|cl| cl.variant.as_str())
                            .unwrap_or("Baseline");
                        let color = VectorColor::new(0.8, 1.8, 0.6, 0.9);
                        self.draw_glyph_at_scale(ctx, ch, variant, center_x, y, scale, color, 1.5);
                    } else if is_current {
                        let uw = advance * 0.7;
                        let pts = [
                            Vec2::new(center_x - uw / 2.0, y + 12.0),
                            Vec2::new(center_x + uw / 2.0, y + 12.0),
                        ];
                        ctx.vectors.stroke_polyline(&pts, 2.5, VectorColor::new(0.8, 0.9, 1.5, 0.8));
                    } else {
                        let uw = advance * 0.6;
                        let pts = [
                            Vec2::new(center_x - uw / 2.0, y + 12.0),
                            Vec2::new(center_x + uw / 2.0, y + 12.0),
                        ];
                        ctx.vectors.stroke_polyline(&pts, 1.5, VectorColor::new(0.4, 0.4, 0.5, 0.4));
                    }

                    cursor_x += advance;
                }

                // Word gap
                cursor_x += gap;
            }
        }
    }

    /// Draw the saying-complete zoom: the full saying scales up and wraps to fill the screen.
    fn draw_saying_zoom(&self, ctx: &mut EngineContext) {
        let total_time = SAYING_CELEBRATE_DURATION;
        let elapsed = total_time - self.celebrate_timer;
        // First 1s: just golden flash, no text yet
        if elapsed < 1.0 {
            return;
        }

        // Zoom from hint scale to moderate scale over 10 seconds
        let zoom_t = ((elapsed - 1.0) / 10.0).min(1.0);
        // Ease-out cubic — slow deceleration
        let eased = 1.0 - (1.0 - zoom_t).powi(3);

        let min_scale = HINT_GLYPH_SCALE;
        let max_scale = 0.30;
        let scale = min_scale + (max_scale - min_scale) * eased;

        let stroke_w = 1.5 + 3.0 * eased;
        let max_width = WORLD_W * 0.90;
        let lines = self.wrap_words_at_scale(scale, max_width);
        let gap = self.word_gap_at_scale(scale);
        let line_h = GLYPH_BASE_H * scale * 1.1;

        // Vertical centering within visible area
        let total_h = lines.len() as f32 * line_h;
        let vis_center_y = self.visible_h / 2.0;
        let start_y = vis_center_y - total_h / 2.0 + line_h / 2.0;

        // Fade in the text color
        let alpha = (zoom_t * 2.0).min(1.0);
        let color = VectorColor::new(1.0 * alpha, 2.2 * alpha, 0.7 * alpha, alpha);

        for (row, (word_indices, line_w)) in lines.iter().enumerate() {
            let y = start_y + row as f32 * line_h;
            let mut cursor_x = (WORLD_W - line_w) / 2.0;

            for &wi in word_indices {
                let word = &self.words[wi];
                for (li, ch) in word.chars().enumerate() {
                    let advance = self.char_advance_at_scale(ch, scale);
                    let center_x = cursor_x + advance / 2.0;

                    let variant = self.completed_letters.iter()
                        .find(|cl| cl.word_idx == wi && cl.letter_idx == li)
                        .map(|cl| cl.variant.as_str())
                        .unwrap_or("Baseline");

                    self.draw_glyph_at_scale(ctx, ch, variant, center_x, y, scale, color, stroke_w);
                    cursor_x += advance;
                }
                cursor_x += gap;
            }
        }
    }

    /// Handle input events.
    fn handle_input(&mut self, input: &InputQueue, ctx: &mut EngineContext) {
        for event in input.iter() {
            match event {
                InputEvent::PointerDown { x, y } => {
                    if self.phase == GamePhase::Tracing {
                        let pos = Vec2::new(*x, *y);
                        match self.tracer.on_pointer_down(pos) {
                            TraceResult::Accepted => {}
                            TraceResult::NotStarted => {
                                // Show a hint — flash the start point
                            }
                            _ => {}
                        }
                    }
                }
                InputEvent::PointerMove { x, y } => {
                    if self.phase == GamePhase::Tracing {
                        let pos = Vec2::new(*x, *y);
                        match self.tracer.on_pointer_move(pos) {
                            TraceResult::Accepted => {}
                            TraceResult::Rejected => {
                                // Save failed stroke for red display
                                if self.tracer.user_points.len() >= 2 {
                                    self.failed_stroke = Some((
                                        self.tracer.user_points.clone(),
                                        FAIL_FADE_TIME,
                                    ));
                                }
                                self.tracer.reset();
                            }
                            TraceResult::StrokeComplete => {
                                let letter_done = self.advance_stroke();
                                if letter_done {
                                    // Spawn celebration particles
                                    let center = self.glyph_center();
                                    ctx.effects.spawn_particles(
                                        [center.x, center.y],
                                        25,
                                        15.0,
                                        5.0,
                                        1.5,
                                    );
                                    self.celebrate_timer = CELEBRATE_DURATION;
                                    self.phase = GamePhase::LetterCelebration;
                                    ctx.emit_event(GameEvent {
                                        kind: EVENT_LETTER_COMPLETE,
                                        a: self.word_idx as f32,
                                        b: self.letter_idx as f32,
                                        c: 0.0,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                InputEvent::PointerUp { .. } => {
                    if self.phase == GamePhase::Tracing {
                        let result = self.tracer.on_pointer_up();
                        if result == TraceResult::Rejected {
                            // Incomplete stroke — save for red display and reset
                            if self.tracer.user_points.len() >= 2 {
                                self.failed_stroke = Some((
                                    self.tracer.user_points.clone(),
                                    FAIL_FADE_TIME,
                                ));
                            }
                            self.tracer.reset();
                        }
                    }
                }
                InputEvent::Custom { kind, a, b, .. } => {
                    if *kind == _CUSTOM_RESTART {
                        self.phase = GamePhase::PickSaying;
                    } else if *kind == CUSTOM_VIEWPORT {
                        // Worker sends visible world dimensions on viewport resize
                        let new_h = *b;
                        if (new_h - self.visible_h).abs() > 1.0 {
                            self.visible_w = *a;
                            self.visible_h = new_h;
                            self.on_viewport_change();
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl Game for Glypher {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: 1.0 / 60.0,
            world_width: WORLD_W,
            world_height: WORLD_H,
            max_instances: 64,
            max_effects_vertices: 32768,
            #[cfg(feature = "vectors")]
            max_vector_vertices: 32768,
            max_lights: 64,
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        // Seed RNG from wall clock so each session is different
        self.rng = Rng::new(js_sys::Date::now() as u64);

        // Parse embedded data
        match BakedGlyphs::from_json(GLYPHS_JSON) {
            Ok(g) => {
                log::info!("Loaded {} glyphs", g.glyphs.len());
                self.glyphs = g;
            }
            Err(e) => log::error!("Failed to parse glyphs: {}", e),
        }

        match SayingsDB::from_json(SAYINGS_JSON) {
            Ok(s) => {
                log::info!("Loaded {} sayings", s.len());
                self.sayings = s;
            }
            Err(e) => log::error!("Failed to parse sayings: {}", e),
        }

        // Spawn background
        self.ensure_background(ctx);

        // Start the first saying
        self.phase = GamePhase::PickSaying;

        log::info!("Glypher initialized");
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        let dt = 1.0 / 60.0;

        // Ensure background stays
        self.ensure_background(ctx);

        // Handle input
        self.handle_input(input, ctx);

        // Update failed stroke fade timer
        if let Some((_, ref mut timer)) = self.failed_stroke {
            *timer -= dt;
            if *timer <= 0.0 {
                self.failed_stroke = None;
            }
        }

        // State machine
        match self.phase {
            GamePhase::PickSaying => {
                self.pick_saying();
                self.show_timer = 0.5;
                self.phase = GamePhase::ShowLetter;
            }

            GamePhase::ShowLetter => {
                self.show_timer -= dt;
                if self.show_timer <= 0.0 {
                    self.phase = GamePhase::Tracing;
                }
            }

            GamePhase::Tracing => {
                // Guide time advances here; light added in rendering section below
                self.guide_time += dt * GUIDE_SPEED;
            }

            GamePhase::LetterCelebration => {
                self.celebrate_timer -= dt;
                if self.celebrate_timer <= 0.0 {
                    self.advance_letter();
                    if self.phase != GamePhase::SayingComplete {
                        self.transition_timer = TRANSITION_DURATION;
                        self.phase = GamePhase::TransitionNext;
                    }
                }
            }

            GamePhase::TransitionNext => {
                self.transition_timer -= dt;
                if self.transition_timer <= 0.0 {
                    self.phase = GamePhase::ShowLetter;
                    self.show_timer = 0.3;
                }
            }

            GamePhase::SayingComplete => {
                self.celebrate_timer -= dt;
                if self.celebrate_timer <= 0.0 {
                    ctx.emit_event(GameEvent {
                        kind: EVENT_SAYING_COMPLETE,
                        a: 0.0,
                        b: 0.0,
                        c: 0.0,
                    });
                    self.phase = GamePhase::PickSaying;
                }
            }
        }

        // --- Rendering ---

        // Clear all lights each frame — we re-add only what's needed this frame
        ctx.lights.clear();

        // Dark ambient — just enough to hint at the leather texture
        ctx.lights.set_ambient(0.10, 0.08, 0.10);

        // Soft fill light so background texture is faintly visible across the whole viewport
        ctx.lights.add(
            PointLight::new(
                Vec2::new(WORLD_W / 2.0, self.visible_h / 2.0),
                [0.12, 0.10, 0.14],
                1.5,
                self.visible_h * 0.7,
            )
        );

        if self.phase == GamePhase::SayingComplete {
            // Saying complete: golden light (no glyph strokes) + zooming text
            self.draw_saying_celebration_light(ctx);
            self.draw_saying_zoom(ctx);
        } else {
            // Normal gameplay rendering

            // Faint static light on the letter area for context
            if self.phase == GamePhase::Tracing || self.phase == GamePhase::ShowLetter {
                self.draw_letter_ambient_light(ctx);
            }

            // Draw guide strokes (faint path indicators)
            if self.phase == GamePhase::Tracing || self.phase == GamePhase::ShowLetter {
                self.draw_guide_strokes(ctx);
            }

            // Draw completed user strokes (green)
            self.draw_completed_strokes(ctx);

            // Draw in-progress user stroke (green)
            self.draw_user_stroke(ctx);

            // Draw failed stroke (red, fading)
            self.draw_failed_stroke(ctx);

            // Dynamic lights: guide (moving), user cursor (follows finger), fail (fading red)
            if self.phase == GamePhase::Tracing || self.phase == GamePhase::ShowLetter {
                self.draw_guide_light(ctx);
            }
            self.draw_user_cursor_light(ctx);
            self.draw_fail_light(ctx);

            // Letter celebration: golden light + golden strokes
            if self.phase == GamePhase::LetterCelebration {
                self.draw_letter_celebration(ctx);
            }

            // Word hints at top
            self.draw_word_hints(ctx);
        }
    }
}
