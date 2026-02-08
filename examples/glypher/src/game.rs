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
const TOP_ZONE_H: f32 = 120.0;
/// Drawing zone: from TOP_ZONE_H to WORLD_H.
const DRAW_ZONE_Y: f32 = TOP_ZONE_H;
const DRAW_ZONE_H: f32 = WORLD_H - TOP_ZONE_H;

/// Glyph rendering area (centered in drawing zone).
const GLYPH_H: f32 = 400.0;
const GLYPH_BASE_W: f32 = 350.0;
const GLYPH_ORIGIN_Y: f32 = DRAW_ZONE_Y + (DRAW_ZONE_H - GLYPH_H) / 2.0;

/// Tracing threshold in world units (~finger width on touchscreen).
const TRACE_THRESHOLD: f32 = 35.0;

/// Guide light speed (path points per second).
const GUIDE_SPEED: f32 = 60.0;

/// Duration of letter celebration (seconds).
const CELEBRATE_DURATION: f32 = 1.2;
/// Duration of transition to next letter (seconds).
const TRANSITION_DURATION: f32 = 0.4;
/// Duration of failed stroke red flash (seconds).
const FAIL_FADE_TIME: f32 = 0.5;

/// Base unit for width-aware hint spacing (one "width unit" in pixels).
const HINT_UNIT: f32 = 14.0;
/// Size of miniature glyphs in top zone.
const HINT_GLYPH_SCALE: f32 = 0.15;
/// Y center of word hints.
const HINT_Y: f32 = TOP_ZONE_H / 2.0;
/// Gap between words in hint zone (pixels).
const HINT_WORD_GAP: f32 = 20.0;

/// Letters that should use High variant at the start of a word.
const WORD_START_HIGH: &[char] = &['m', 'n', 'v', 'w'];

// --- Game event kinds (Rust → React) ---
const EVENT_SAYING_COMPLETE: f32 = 1.0;
const EVENT_LETTER_COMPLETE: f32 = 2.0;

// --- Custom event kinds (React → Rust) ---
const _CUSTOM_RESTART: u32 = 1;

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
    ch: char,
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

    // Saying index counter (for picking different sayings)
    saying_counter: usize,

    phase: GamePhase,

    // Background entity spawned flag
    bg_spawned: bool,
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
            saying_counter: 0,

            phase: GamePhase::PickSaying,
            bg_spawned: false,
        }
    }

    /// Convert glyph normalized coordinates [0,1] to world coordinates,
    /// taking into account the letter's width class.
    fn glyph_to_world(&self, gx: f32, gy: f32, width: u8) -> Vec2 {
        let w_scale = match width {
            0 => GLYPH_BASE_W * 0.5,  // Narrow
            2 => GLYPH_BASE_W * 1.3,  // Wide
            _ => GLYPH_BASE_W,        // Standard
        };
        let origin_x = (WORLD_W - w_scale) / 2.0;
        Vec2::new(
            gx * w_scale + origin_x,
            gy * GLYPH_H + GLYPH_ORIGIN_Y,
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

    /// Pick a new saying and reset all letter state.
    fn pick_saying(&mut self) {
        self.current_saying = self.sayings.pick(self.saying_counter).to_string();
        self.saying_counter += 1;
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
            self.tracer = StrokeTracer::new(stroke_world.clone(), TRACE_THRESHOLD);
            self.guide_time = 0.0;
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
            ch,
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
                self.celebrate_timer = CELEBRATE_DURATION * 2.0;
                return;
            }
        }

        self.setup_current_letter();
    }

    /// Spawn the background entity (once).
    fn ensure_background(&mut self, ctx: &mut EngineContext) {
        if self.bg_spawned {
            return;
        }
        self.bg_spawned = true;

        let bg_id = ctx.next_id();
        if let Some(bg_sprite) = ctx.sprite("bg") {
            ctx.scene.spawn(
                Entity::new(bg_id)
                    .with_tag("bg")
                    .with_pos(Vec2::new(WORLD_W / 2.0, WORLD_H / 2.0))
                    .with_scale(Vec2::new(WORLD_W, WORLD_H))
                    .with_layer(RenderLayer::Background)
                    .with_sprite(bg_sprite),
            );
        }
    }

    /// Draw guide strokes (faint blue) for the current letter.
    fn draw_guide_strokes(&self, ctx: &mut EngineContext) {
        let guide_color = VectorColor::new(0.3, 0.5, 0.8, 0.12);
        for (i, stroke) in self.current_strokes_world.iter().enumerate() {
            if stroke.len() < 2 {
                continue;
            }
            // Already-completed strokes are dimmer
            let color = if i < self.stroke_idx {
                VectorColor::new(0.2, 0.3, 0.5, 0.06)
            } else {
                guide_color
            };
            ctx.vectors.stroke_polyline(stroke, 6.0, color);
        }
    }

    /// Draw and update the guide light that traces along the current stroke.
    fn update_guide_light(&mut self, ctx: &mut EngineContext, dt: f32) {
        if let Some(stroke) = self.current_strokes_world.get(self.stroke_idx) {
            if stroke.is_empty() {
                return;
            }

            self.guide_time += dt * GUIDE_SPEED;
            let total = stroke.len() as f32;
            // Ping-pong: forward then pause then forward again
            let t = self.guide_time % (total + 30.0); // 30 frames pause at end
            let idx = (t as usize).min(stroke.len() - 1);
            let pos = stroke[idx];

            // Guide light — soft blue
            ctx.lights.add(
                PointLight::new(pos, [0.3, 0.5, 1.0], 1.0, 100.0)
            );

            // Glow dot at guide position
            ctx.vectors.fill_circle(pos, 8.0, VectorColor::new(0.5, 0.7, 1.5, 0.6));
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

    /// Place lights along user's accepted strokes for background illumination.
    fn draw_user_lights(&self, ctx: &mut EngineContext) {
        let light_color = [0.15, 0.6, 0.25];
        let intensity = 0.6;
        let radius = 70.0;

        // Completed strokes
        for stroke in &self.completed_strokes {
            for chunk in stroke.points.chunks(12) {
                if let Some(last) = chunk.last() {
                    ctx.lights.add(PointLight::new(*last, light_color, intensity, radius));
                }
            }
        }

        // In-progress stroke
        if self.tracer.user_points.len() > 1 {
            for chunk in self.tracer.user_points.chunks(8) {
                if let Some(last) = chunk.last() {
                    ctx.lights.add(PointLight::new(*last, light_color, intensity, radius));
                }
            }
        }
    }

    /// Draw letter celebration effects.
    fn draw_celebration(&self, ctx: &mut EngineContext) {
        let t = self.celebrate_timer / CELEBRATE_DURATION;
        if t <= 0.0 {
            return;
        }

        let gold = VectorColor::new(8.0 * t, 6.8 * t, 2.4 * t, 1.0);
        let light_intensity = 6.0 * t;
        let stroke_width = 8.0 + 12.0 * t;

        // Draw all strokes as golden, wide, glowing
        for stroke in &self.current_strokes_world {
            if stroke.len() >= 2 {
                ctx.vectors.stroke_polyline(stroke, stroke_width, gold);
            }
        }

        // Golden lights along all strokes
        for stroke in &self.current_strokes_world {
            for (i, point) in stroke.iter().enumerate() {
                if i % 6 == 0 {
                    ctx.lights.add(
                        PointLight::new(*point, [1.0, 0.85, 0.3], light_intensity, 120.0)
                    );
                }
            }
        }
    }

    /// Compute the advance width for a character in hint-zone pixels.
    /// Each letter occupies: 0.5 + width(0,1,2) + 0.5 = (width + 1) width units.
    fn hint_advance(&self, ch: char) -> f32 {
        let w = self.glyphs.width(ch) as f32;
        (w + 1.0) * HINT_UNIT
    }

    /// Compute total hint-zone pixel width for a word (sum of all letter advances).
    fn hint_word_width(&self, word: &str) -> f32 {
        word.chars().map(|ch| self.hint_advance(ch)).sum()
    }

    /// Compute total hint-zone width for the full saying (all words + gaps).
    fn hint_total_width(&self) -> f32 {
        let mut total = 0.0;
        for (i, word) in self.words.iter().enumerate() {
            total += self.hint_word_width(word);
            if i + 1 < self.words.len() {
                total += HINT_WORD_GAP;
            }
        }
        total
    }

    /// Draw word hints at the top of the screen.
    fn draw_word_hints(&self, ctx: &mut EngineContext) {
        // Layout all words centered across the screen
        let total_w = self.hint_total_width();
        let mut cursor_x = (WORLD_W - total_w) / 2.0;

        for (wi, word) in self.words.iter().enumerate() {
            for (li, ch) in word.chars().enumerate() {
                let advance = self.hint_advance(ch);
                let center_x = cursor_x + advance / 2.0;

                let is_completed = self.completed_letters.iter().any(|cl| {
                    cl.word_idx == wi && cl.letter_idx == li
                });
                let is_current = wi == self.word_idx && li == self.letter_idx;
                let is_past_word = wi < self.word_idx;

                if is_completed || is_past_word {
                    // Draw miniature glyph — look up the exact variant used
                    let variant = self.completed_letters.iter()
                        .find(|cl| cl.word_idx == wi && cl.letter_idx == li)
                        .map(|cl| cl.variant.as_str())
                        .unwrap_or("Baseline");
                    self.draw_mini_glyph(ctx, ch, variant, center_x, HINT_Y);
                } else if is_current {
                    // Highlight current letter underscore
                    let uw = advance * 0.7;
                    let pts = [
                        Vec2::new(center_x - uw / 2.0, HINT_Y + 12.0),
                        Vec2::new(center_x + uw / 2.0, HINT_Y + 12.0),
                    ];
                    ctx.vectors.stroke_polyline(&pts, 3.0, VectorColor::new(0.8, 0.9, 1.5, 0.8));
                } else {
                    // Dim underscore for pending letters
                    let uw = advance * 0.6;
                    let pts = [
                        Vec2::new(center_x - uw / 2.0, HINT_Y + 12.0),
                        Vec2::new(center_x + uw / 2.0, HINT_Y + 12.0),
                    ];
                    ctx.vectors.stroke_polyline(&pts, 2.0, VectorColor::new(0.4, 0.4, 0.5, 0.4));
                }

                cursor_x += advance;
            }

            // Word gap
            if wi + 1 < self.words.len() {
                cursor_x += HINT_WORD_GAP;
            }
        }
    }

    /// Draw a miniature version of a glyph at a given position.
    fn draw_mini_glyph(&self, ctx: &mut EngineContext, ch: char, variant: &str, center_x: f32, center_y: f32) {
        let width = self.glyphs.width(ch);
        if let Some(strokes) = self.glyphs.get_strokes(ch, variant) {
            let scale = HINT_GLYPH_SCALE;
            let w_scale = match width {
                0 => GLYPH_BASE_W * 0.5,
                2 => GLYPH_BASE_W * 1.3,
                _ => GLYPH_BASE_W,
            } * scale;
            let h_scale = GLYPH_H * scale;

            let color = VectorColor::new(0.8, 1.8, 0.6, 0.9);
            for stroke in strokes {
                let world_pts: Vec<Vec2> = stroke.iter()
                    .map(|p| Vec2::new(
                        center_x + (p[0] - 0.5) * w_scale,
                        center_y + (p[1] - 0.5) * h_scale,
                    ))
                    .collect();
                if world_pts.len() >= 2 {
                    ctx.vectors.stroke_polyline(&world_pts, 3.0, color);
                }
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
                                    let center = Vec2::new(WORLD_W / 2.0, DRAW_ZONE_Y + DRAW_ZONE_H / 2.0);
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
                InputEvent::Custom { kind, .. } => {
                    if *kind == _CUSTOM_RESTART {
                        self.phase = GamePhase::PickSaying;
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
                // Guide light animation
                self.update_guide_light(ctx, dt);
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

        // Dark ambient lighting
        ctx.lights.set_ambient(0.06, 0.05, 0.07);

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

        // User stroke lights
        self.draw_user_lights(ctx);

        // Celebration effects
        if self.phase == GamePhase::LetterCelebration || self.phase == GamePhase::SayingComplete {
            self.draw_celebration(ctx);
        }

        // Word hints at top
        self.draw_word_hints(ctx);
    }
}
