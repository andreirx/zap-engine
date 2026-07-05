use glam::Vec2;
use serde::{Deserialize, Serialize};
use zap_engine::api::game::GameConfig;
use zap_engine::components::sprite::BlendMode;
use zap_engine::input::queue::{InputEvent, InputQueue};
use zap_engine::systems::vector::VectorColor;
use zap_engine::{EngineContext, Entity, Game, GameEvent, RenderLayer};

const VIEW_W: f32 = 1600.0;
const VIEW_H: f32 = 900.0;
const TILE: f32 = 128.0;
const LEVEL_COLS: usize = 64;
const LEVEL_ROWS: usize = 32;
const LEVEL_W: f32 = LEVEL_COLS as f32 * TILE;
const LEVEL_H: f32 = LEVEL_ROWS as f32 * TILE;

const EDIT_PADDING: f32 = 32.0;
const PLAY_VISIBLE_TILES: f32 = 10.0;
const CAMERA_LERP: f32 = 0.14;

const PLAYER_HALF_W: f32 = 42.0;
const PLAYER_HALF_H: f32 = 58.0;
const MOVE_ACCEL: f32 = 2400.0;
const MOVE_DECEL: f32 = 1800.0;
const MAX_RUN_SPEED: f32 = 520.0;
const GRAVITY: f32 = 2200.0;
const STANDING_JUMP_SPEED: f32 = 900.0;
const RUNNING_JUMP_SPEED: f32 = 1250.0;
const RUNNING_JUMP_THRESHOLD: f32 = 280.0;
const WATER_GRAVITY: f32 = 280.0;
const WATER_HORIZONTAL_ACCEL: f32 = 900.0;
const WATER_DRAG_X: f32 = 0.88;
const WATER_DRAG_Y: f32 = 0.82;
const WATER_JUMP_IMPULSE: f32 = 420.0;
const WATER_MAX_UP_SPEED: f32 = 620.0;
const WATER_MAX_DOWN_SPEED: f32 = 180.0;
const FALL_RETURN_MARGIN: f32 = TILE * 2.0;
const MONEY_PICKUP_RADIUS: f32 = 70.0;
const ANIM_FRAME_TIME: f32 = 0.11;
const WATER_ANIM_RATE: f32 = 0.45;
const FIXED_DT: f32 = 1.0 / 60.0;

const VOLCANO_TILES: f32 = 8.0;
const VOLCANO_SIZE: f32 = VOLCANO_TILES * TILE;
const FIREBALL_SIZE: f32 = TILE;
const FIREBALL_HIT_RADIUS: f32 = 54.0;
const FIREBALL_HDR_ALPHA: f32 = 4.0;
const SMOKE_FRAMES: u32 = 128;
const ERUPTION_MIN_FRAMES: u32 = 0;
const ERUPTION_MAX_EXTRA_FRAMES: u32 = 2 * 60 + 1;
const ERUPTION_DURATION_FRAMES: u32 = 4 * 60;
const GARGLE_FIREBALL_COUNT: usize = 4;

const EVENT_MODE: f32 = 1.0;
const EVENT_SCORE: f32 = 2.0;

const CUSTOM_SET_TOOL: u32 = 1;
const CUSTOM_SET_ACTION: u32 = 2;
const CUSTOM_SET_RECTANGLE: u32 = 3;
const CUSTOM_SET_DINO_COLOR: u32 = 4;
const CUSTOM_PLAY: u32 = 5;
const CUSTOM_EDIT: u32 = 6;
const CUSTOM_RESET_LEVEL: u32 = 7;
const CUSTOM_RESIZE: u32 = 99;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Edit,
    Play,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditTool {
    Ground,
    Money1,
    Money10,
    DinoStart,
    Finish,
    Volcano,
    Lava,
    Water,
}

impl EditTool {
    fn from_code(code: f32) -> Self {
        match code.round() as i32 {
            1 => Self::Money1,
            2 => Self::Money10,
            3 => Self::DinoStart,
            4 => Self::Finish,
            5 => Self::Volcano,
            6 => Self::Lava,
            7 => Self::Water,
            _ => Self::Ground,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditAction {
    Place,
    Erase,
}

impl EditAction {
    fn from_code(code: f32) -> Self {
        if code.round() as i32 == 1 {
            Self::Erase
        } else {
            Self::Place
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DinoColor {
    Verde,
    Albastru,
    Galben,
    Mov,
    Rosu,
}

impl DinoColor {
    fn from_code(code: f32) -> Self {
        match code.round() as i32 {
            1 => Self::Albastru,
            2 => Self::Galben,
            3 => Self::Mov,
            4 => Self::Rosu,
            _ => Self::Verde,
        }
    }

    fn from_save_code(code: u8) -> Self {
        match code {
            1 => Self::Albastru,
            2 => Self::Galben,
            3 => Self::Mov,
            4 => Self::Rosu,
            _ => Self::Verde,
        }
    }

    fn save_code(self) -> u8 {
        match self {
            Self::Verde => 0,
            Self::Albastru => 1,
            Self::Galben => 2,
            Self::Mov => 3,
            Self::Rosu => 4,
        }
    }

    fn stem(self) -> &'static str {
        match self {
            Self::Verde => "dino-verde",
            Self::Albastru => "dino-albastru",
            Self::Galben => "dino-galben",
            Self::Mov => "dino-mov",
            Self::Rosu => "dino-rosu",
        }
    }

    fn sprite_name(self, frame: usize) -> String {
        format!("{}-{}", self.stem(), frame.clamp(1, 4))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroundSprite {
    Surface,
    Adanc,
    Pietros,
    PietrosDiamant,
}

impl GroundSprite {
    fn name(self) -> &'static str {
        match self {
            Self::Surface => "pamant",
            Self::Adanc => "pamant-adanc",
            Self::Pietros => "pamant-pietros",
            Self::PietrosDiamant => "pamant-pietros-diamant",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TileKind {
    Empty,
    Ground,
    Lava,
    Water,
}

impl TileKind {
    fn from_save_code(code: u8) -> Self {
        match code {
            1 => Self::Ground,
            2 => Self::Lava,
            3 => Self::Water,
            _ => Self::Empty,
        }
    }

    fn save_code(self) -> u8 {
        match self {
            Self::Empty => 0,
            Self::Ground => 1,
            Self::Lava => 2,
            Self::Water => 3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TileCell {
    kind: TileKind,
    sprite: GroundSprite,
}

impl TileCell {
    const fn empty() -> Self {
        Self {
            kind: TileKind::Empty,
            sprite: GroundSprite::Surface,
        }
    }
}

#[derive(Debug, Clone)]
struct Money {
    col: usize,
    row: usize,
    value: u32,
    collected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LevelDocument {
    version: Option<u32>,
    cols: Option<usize>,
    rows: Option<usize>,
    tiles: Option<Vec<u8>>,
    ground: Option<Vec<bool>>,
    money: Option<Vec<LevelMoney>>,
    dino_start: Option<[f32; 2]>,
    dino_color: Option<u8>,
    finish: Option<[usize; 2]>,
    volcano: Option<[usize; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LevelMoney {
    col: usize,
    row: usize,
    value: u32,
}

#[derive(Debug, Clone, Copy)]
struct CameraTransform {
    left: f32,
    top: f32,
    scale: f32,
}

impl CameraTransform {
    fn world_to_screen(self, p: Vec2) -> Vec2 {
        Vec2::new(
            (p.x - self.left) * self.scale,
            (p.y - self.top) * self.scale,
        )
    }

    fn screen_to_world(self, p: Vec2) -> Vec2 {
        Vec2::new(p.x / self.scale + self.left, p.y / self.scale + self.top)
    }

    fn lerp_toward(&mut self, target: Self) {
        self.left += (target.left - self.left) * CAMERA_LERP;
        self.top += (target.top - self.top) * CAMERA_LERP;
        self.scale += (target.scale - self.scale) * CAMERA_LERP;
    }
}

#[derive(Debug, Clone, Copy)]
struct PlayerState {
    pos: Vec2,
    vel: Vec2,
    on_ground: bool,
    anim_time: f32,
}

#[derive(Debug, Clone, Copy)]
struct LoopingFireball {
    phase: f32,
    period_frames: f32,
    x_offset: f32,
    height: f32,
}

#[derive(Debug, Clone, Copy)]
struct ActiveEruption {
    frame: u32,
    duration_frames: u32,
    start: Vec2,
    apex: Vec2,
    target: Vec2,
    pos: Vec2,
    prev_pos: Vec2,
}

#[derive(Debug, Clone, Copy)]
struct SmokePuff {
    pos: Vec2,
    age_frames: u32,
    rotation: f32,
    size: f32,
}

pub struct DinoLevelEditor {
    tiles: Vec<TileCell>,
    money: Vec<Money>,
    mode: Mode,
    edit_tool: EditTool,
    edit_action: EditAction,
    rectangle_mode: bool,
    dragging: bool,
    rect_start: Option<(usize, usize)>,
    last_cell: Option<(usize, usize)>,
    dino_color: DinoColor,
    dino_start: Vec2,
    player: PlayerState,
    finish_col: usize,
    finish_row: usize,
    volcano_col: usize,
    volcano_row: usize,
    keys: KeyState,
    score: u32,
    viewport_w: f32,
    viewport_h: f32,
    camera: CameraTransform,
    play_camera_left: f32,
    last_emitted_mode: Mode,
    last_emitted_score: u32,
    gargle_fireballs: [LoopingFireball; GARGLE_FIREBALL_COUNT],
    eruptions: Vec<ActiveEruption>,
    smoke: Vec<SmokePuff>,
    eruption_timer_frames: u32,
    hazard_rng: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct KeyState {
    right: bool,
    jump: bool,
    jump_pressed: bool,
}

impl DinoLevelEditor {
    pub fn new() -> Self {
        let mut editor = Self {
            tiles: vec![TileCell::empty(); LEVEL_COLS * LEVEL_ROWS],
            money: Vec::new(),
            mode: Mode::Edit,
            edit_tool: EditTool::Ground,
            edit_action: EditAction::Place,
            rectangle_mode: false,
            dragging: false,
            rect_start: None,
            last_cell: None,
            dino_color: DinoColor::Verde,
            dino_start: Vec2::new(3.4 * TILE, 0.0),
            player: PlayerState {
                pos: Vec2::ZERO,
                vel: Vec2::ZERO,
                on_ground: false,
                anim_time: 0.0,
            },
            finish_col: 56,
            finish_row: 23,
            volcano_col: 28,
            volcano_row: 16,
            keys: KeyState::default(),
            score: 0,
            viewport_w: VIEW_W,
            viewport_h: VIEW_H,
            camera: Self::edit_camera_for(VIEW_W, VIEW_H),
            play_camera_left: 0.0,
            last_emitted_mode: Mode::Edit,
            last_emitted_score: 0,
            gargle_fireballs: Self::initial_gargle_fireballs(),
            eruptions: Vec::with_capacity(8),
            smoke: Vec::with_capacity(800),
            eruption_timer_frames: ERUPTION_MIN_FRAMES,
            hazard_rng: 0x1234_abcd,
        };
        editor.reset_to_seed_level();
        editor
    }

    fn idx(col: usize, row: usize) -> usize {
        row * LEVEL_COLS + col
    }

    fn in_bounds(col: isize, row: isize) -> bool {
        col >= 0 && row >= 0 && col < LEVEL_COLS as isize && row < LEVEL_ROWS as isize
    }

    fn tile(&self, col: usize, row: usize) -> TileCell {
        self.tiles[Self::idx(col, row)]
    }

    fn tile_mut(&mut self, col: usize, row: usize) -> &mut TileCell {
        let idx = Self::idx(col, row);
        &mut self.tiles[idx]
    }

    pub fn reset_to_seed_level(&mut self) {
        for tile in &mut self.tiles {
            *tile = TileCell::empty();
        }
        self.money.clear();
        self.mode = Mode::Edit;
        self.edit_tool = EditTool::Ground;
        self.edit_action = EditAction::Place;
        self.rectangle_mode = false;
        self.dragging = false;
        self.rect_start = None;
        self.last_cell = None;
        self.dino_color = DinoColor::Verde;
        self.dino_start = Vec2::new(3.4 * TILE, 0.0);
        self.finish_col = 57;
        self.finish_row = 23;
        self.volcano_col = 28;
        self.volcano_row = 16;
        self.keys = KeyState::default();
        self.score = 0;
        self.play_camera_left = 0.0;
        self.eruptions.clear();
        self.smoke.clear();
        self.eruption_timer_frames = ERUPTION_MIN_FRAMES;
        self.seed_initial_level();
        self.dino_start.y = self.surface_y_at_x(self.dino_start.x) - PLAYER_HALF_H;
        self.reset_player_to_start();
    }

    pub fn export_level_json(&self) -> String {
        let document = LevelDocument {
            version: Some(1),
            cols: Some(LEVEL_COLS),
            rows: Some(LEVEL_ROWS),
            tiles: Some(
                self.tiles
                    .iter()
                    .map(|tile| tile.kind.save_code())
                    .collect(),
            ),
            ground: None,
            money: Some(
                self.money
                    .iter()
                    .map(|money| LevelMoney {
                        col: money.col,
                        row: money.row,
                        value: money.value,
                    })
                    .collect(),
            ),
            dino_start: Some([self.dino_start.x, self.dino_start.y]),
            dino_color: Some(self.dino_color.save_code()),
            finish: Some([self.finish_col, self.finish_row]),
            volcano: Some([self.volcano_col, self.volcano_row]),
        };
        serde_json::to_string(&document).unwrap_or_else(|err| {
            log::warn!("Failed to serialize dino level: {err}");
            "{}".to_string()
        })
    }

    pub fn load_level_json(&mut self, json: &str) -> Result<(), String> {
        let document: LevelDocument =
            serde_json::from_str(json).map_err(|err| format!("invalid level JSON: {err}"))?;
        self.apply_level_document(document);
        Ok(())
    }

    fn apply_level_document(&mut self, document: LevelDocument) {
        self.reset_to_seed_level();

        if let Some(tiles) = document.tiles {
            for tile in &mut self.tiles {
                *tile = TileCell::empty();
            }
            for (idx, code) in tiles.iter().copied().enumerate().take(self.tiles.len()) {
                self.tiles[idx].kind = TileKind::from_save_code(code);
            }
        } else if let Some(ground) = document.ground {
            for tile in &mut self.tiles {
                *tile = TileCell::empty();
            }
            for (idx, is_ground) in ground.iter().copied().enumerate().take(self.tiles.len()) {
                if is_ground {
                    self.tiles[idx].kind = TileKind::Ground;
                }
            }
        }

        self.money.clear();
        if let Some(money) = document.money {
            for item in money {
                if item.col < LEVEL_COLS && item.row < LEVEL_ROWS {
                    self.money.push(Money {
                        col: item.col,
                        row: item.row,
                        value: if item.value == 10 { 10 } else { 1 },
                        collected: false,
                    });
                }
            }
        }

        if let Some([x, y]) = document.dino_start {
            self.dino_start = Vec2::new(
                x.clamp(PLAYER_HALF_W, LEVEL_W - PLAYER_HALF_W),
                y.clamp(PLAYER_HALF_H, LEVEL_H - PLAYER_HALF_H),
            );
        }
        if let Some(color) = document.dino_color {
            self.dino_color = DinoColor::from_save_code(color);
        }
        if let Some([col, row]) = document.finish {
            self.finish_col = col.min(LEVEL_COLS - 2);
            self.finish_row = row.min(LEVEL_ROWS - 1);
        }
        if let Some([col, row]) = document.volcano {
            self.volcano_col = col.min(LEVEL_COLS - VOLCANO_TILES as usize);
            self.volcano_row = row.min(LEVEL_ROWS - VOLCANO_TILES as usize);
        }

        self.mode = Mode::Edit;
        self.keys = KeyState::default();
        self.score = 0;
        self.eruptions.clear();
        self.smoke.clear();
        self.eruption_timer_frames = 0;
        self.recompute_ground_sprites();
        self.reset_player_to_start();
    }

    fn seed_initial_level(&mut self) {
        for col in 0..10 {
            for row in 24..LEVEL_ROWS {
                self.tile_mut(col, row).kind = TileKind::Ground;
            }
        }
        for col in 54..62 {
            for row in 24..LEVEL_ROWS {
                self.tile_mut(col, row).kind = TileKind::Ground;
            }
        }
        self.finish_col = 57;
        self.finish_row = 23;
        self.recompute_ground_sprites();
    }

    fn initial_gargle_fireballs() -> [LoopingFireball; GARGLE_FIREBALL_COUNT] {
        [
            LoopingFireball {
                phase: 0.00,
                period_frames: 82.0,
                x_offset: -TILE * 0.36,
                height: TILE * 1.55,
            },
            LoopingFireball {
                phase: 0.28,
                period_frames: 96.0,
                x_offset: TILE * 0.18,
                height: TILE * 2.05,
            },
            LoopingFireball {
                phase: 0.52,
                period_frames: 74.0,
                x_offset: TILE * 0.46,
                height: TILE * 1.35,
            },
            LoopingFireball {
                phase: 0.73,
                period_frames: 108.0,
                x_offset: -TILE * 0.06,
                height: TILE * 1.85,
            },
        ]
    }

    fn volcano_center(&self) -> Vec2 {
        Vec2::new(
            self.volcano_col as f32 * TILE + VOLCANO_SIZE * 0.5,
            self.volcano_row as f32 * TILE + VOLCANO_SIZE * 0.5,
        )
    }

    fn crater_pos(&self) -> Vec2 {
        let center = self.volcano_center();
        Vec2::new(center.x, center.y - VOLCANO_SIZE * 0.34)
    }

    fn next_random_u32(&mut self) -> u32 {
        let mut x = self.hazard_rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.hazard_rng = x;
        x
    }

    fn next_eruption_delay(&mut self) -> u32 {
        ERUPTION_MIN_FRAMES + (self.next_random_u32() % ERUPTION_MAX_EXTRA_FRAMES)
    }

    fn start_eruption(&mut self) {
        let start = self.crater_pos();
        let target_x =
            (self.next_random_u32() as f32 / u32::MAX as f32) * (LEVEL_W - TILE) + TILE * 0.5;
        let target_y = self.surface_y_at_x(target_x) - FIREBALL_SIZE * 0.5;
        let apex = Vec2::new((start.x + target_x) * 0.5, TILE * 0.35);
        self.eruptions.push(ActiveEruption {
            frame: 0,
            duration_frames: ERUPTION_DURATION_FRAMES,
            start,
            apex,
            target: Vec2::new(target_x, target_y),
            pos: start,
            prev_pos: start,
        });
        self.eruption_timer_frames = self.next_eruption_delay();
    }

    fn bezier2(a: Vec2, b: Vec2, c: Vec2, t: f32) -> Vec2 {
        let inv = 1.0 - t;
        a * inv * inv + b * 2.0 * inv * t + c * t * t
    }

    fn fireball_rotation(velocity: Vec2) -> f32 {
        if velocity.length_squared() < 0.001 {
            0.0
        } else {
            velocity.x.atan2(-velocity.y) + std::f32::consts::PI
        }
    }

    fn gargle_position(crater: Vec2, fireball: LoopingFireball) -> (Vec2, Vec2) {
        let t = fireball.phase.fract();
        let angle = std::f32::consts::TAU * t;
        let arc = std::f32::consts::PI * t;
        let pos = crater
            + Vec2::new(
                fireball.x_offset * angle.sin(),
                -fireball.height * arc.sin(),
            );
        let vel = Vec2::new(
            fireball.x_offset * angle.cos(),
            -fireball.height * arc.cos(),
        );
        (pos, vel)
    }

    fn recompute_ground_sprites(&mut self) {
        for row in 0..LEVEL_ROWS {
            for col in 0..LEVEL_COLS {
                let is_ground = self.tile(col, row).kind == TileKind::Ground;
                if !is_ground {
                    continue;
                }
                let has_ground_above = row > 0 && self.tile(col, row - 1).kind == TileKind::Ground;
                let sprite = if !has_ground_above {
                    GroundSprite::Surface
                } else {
                    match Self::variation_seed(col, row) % 3 {
                        0 => GroundSprite::Adanc,
                        1 => GroundSprite::Pietros,
                        _ => GroundSprite::PietrosDiamant,
                    }
                };
                self.tile_mut(col, row).sprite = sprite;
            }
        }
    }

    fn variation_seed(col: usize, row: usize) -> u32 {
        let mut x = (col as u32).wrapping_mul(0x9e37_79b9) ^ (row as u32).wrapping_mul(0x85eb_ca6b);
        x ^= x >> 16;
        x = x.wrapping_mul(0x7feb_352d);
        x ^ (x >> 15)
    }

    fn cell_from_world(p: Vec2) -> Option<(usize, usize)> {
        let col = (p.x / TILE).floor() as isize;
        let row = (p.y / TILE).floor() as isize;
        if Self::in_bounds(col, row) {
            Some((col as usize, row as usize))
        } else {
            None
        }
    }

    fn tile_center(col: usize, row: usize) -> Vec2 {
        Vec2::new(
            col as f32 * TILE + TILE * 0.5,
            row as f32 * TILE + TILE * 0.5,
        )
    }

    fn surface_y_at_x(&self, x: f32) -> f32 {
        let col = (x / TILE).floor().clamp(0.0, (LEVEL_COLS - 1) as f32) as usize;
        for row in 0..LEVEL_ROWS {
            if self.tile(col, row).kind == TileKind::Ground {
                return row as f32 * TILE;
            }
        }
        LEVEL_H
    }

    fn place_ground(&mut self, col: usize, row: usize) {
        self.tile_mut(col, row).kind = TileKind::Ground;
        self.recompute_ground_sprites();
    }

    fn erase_ground(&mut self, col: usize, row: usize) {
        self.tile_mut(col, row).kind = TileKind::Empty;
        self.money.retain(|m| !(m.col == col && m.row == row));
        self.recompute_ground_sprites();
    }

    fn place_lava(&mut self, col: usize, row: usize) {
        self.tile_mut(col, row).kind = TileKind::Lava;
        self.recompute_ground_sprites();
    }

    fn place_water(&mut self, col: usize, row: usize) {
        self.tile_mut(col, row).kind = TileKind::Water;
        self.recompute_ground_sprites();
    }

    fn place_money(&mut self, col: usize, row: usize, value: u32) {
        if self.money.iter().any(|m| m.col == col && m.row == row) {
            return;
        }
        self.money.push(Money {
            col,
            row,
            value,
            collected: false,
        });
    }

    fn erase_money(&mut self, col: usize, row: usize) {
        self.money.retain(|m| !(m.col == col && m.row == row));
    }

    fn place_dino_start(&mut self, world: Vec2) {
        let x = world.x.clamp(PLAYER_HALF_W, LEVEL_W - PLAYER_HALF_W);
        let y = self.surface_y_at_x(x) - PLAYER_HALF_H;
        self.dino_start = Vec2::new(x, y);
        if self.mode == Mode::Edit {
            self.reset_player_to_start();
        }
    }

    fn place_finish(&mut self, col: usize, row: usize) {
        let surface_row = if self.tile(col, row).kind == TileKind::Ground {
            row
        } else if row + 1 < LEVEL_ROWS && self.tile(col, row + 1).kind == TileKind::Ground {
            row + 1
        } else {
            return;
        };
        let top_row = surface_row.saturating_sub(1);
        self.finish_col = col.min(LEVEL_COLS - 2);
        self.finish_row = top_row;
    }

    fn place_volcano(&mut self, world: Vec2) {
        let max_col = LEVEL_COLS - VOLCANO_TILES as usize;
        let max_row = LEVEL_ROWS - VOLCANO_TILES as usize;
        let top_left_col = ((world.x - VOLCANO_SIZE * 0.5) / TILE).round() as isize;
        let top_left_row = ((world.y - VOLCANO_SIZE * 0.5) / TILE).round() as isize;
        self.volcano_col = top_left_col.clamp(0, max_col as isize) as usize;
        self.volcano_row = top_left_row.clamp(0, max_row as isize) as usize;

        // Existing smoke/fireball positions belong to the old crater; drop them so
        // the hazard resumes cleanly from the moved volcano on the next update.
        self.smoke.clear();
        self.eruptions.clear();
        self.eruption_timer_frames = 0;
    }

    fn apply_piecewise_edit(&mut self, world: Vec2) {
        let Some((col, row)) = Self::cell_from_world(world) else {
            return;
        };
        match (self.edit_action, self.edit_tool) {
            (EditAction::Place, EditTool::Ground) => self.place_ground(col, row),
            (EditAction::Erase, EditTool::Ground) => self.erase_ground(col, row),
            (EditAction::Place, EditTool::Lava) => self.place_lava(col, row),
            (EditAction::Erase, EditTool::Lava) => self.erase_ground(col, row),
            (EditAction::Place, EditTool::Water) => self.place_water(col, row),
            (EditAction::Erase, EditTool::Water) => self.erase_ground(col, row),
            (EditAction::Place, EditTool::Money1) => self.place_money(col, row, 1),
            (EditAction::Place, EditTool::Money10) => self.place_money(col, row, 10),
            (EditAction::Erase, EditTool::Money1 | EditTool::Money10) => self.erase_money(col, row),
            (EditAction::Place, EditTool::DinoStart) => self.place_dino_start(world),
            (EditAction::Erase, EditTool::DinoStart) => {}
            (EditAction::Place, EditTool::Finish) => self.place_finish(col, row),
            (EditAction::Erase, EditTool::Finish) => {}
            (EditAction::Place, EditTool::Volcano) => self.place_volcano(world),
            (EditAction::Erase, EditTool::Volcano) => {}
        }
    }

    fn apply_rectangle_edit(&mut self, end_col: usize, end_row: usize) {
        let Some((start_col, start_row)) = self.rect_start else {
            return;
        };
        let min_col = start_col.min(end_col);
        let max_col = start_col.max(end_col);
        let min_row = start_row.min(end_row);
        let max_row = start_row.max(end_row);
        for row in min_row..=max_row {
            for col in min_col..=max_col {
                match (self.edit_action, self.edit_tool) {
                    (EditAction::Place, EditTool::Ground) => {
                        self.tile_mut(col, row).kind = TileKind::Ground
                    }
                    (EditAction::Erase, EditTool::Ground) => {
                        self.tile_mut(col, row).kind = TileKind::Empty
                    }
                    (EditAction::Place, EditTool::Lava) => {
                        self.tile_mut(col, row).kind = TileKind::Lava
                    }
                    (EditAction::Erase, EditTool::Lava) => {
                        self.tile_mut(col, row).kind = TileKind::Empty
                    }
                    (EditAction::Place, EditTool::Water) => {
                        self.tile_mut(col, row).kind = TileKind::Water
                    }
                    (EditAction::Erase, EditTool::Water) => {
                        self.tile_mut(col, row).kind = TileKind::Empty
                    }
                    (EditAction::Place, EditTool::Money1) => self.place_money(col, row, 1),
                    (EditAction::Place, EditTool::Money10) => self.place_money(col, row, 10),
                    (EditAction::Erase, EditTool::Money1 | EditTool::Money10) => {
                        self.erase_money(col, row)
                    }
                    _ => {}
                }
            }
        }
        if matches!(
            self.edit_tool,
            EditTool::Ground | EditTool::Lava | EditTool::Water
        ) {
            let tiles = &self.tiles;
            let action = self.edit_action;
            self.money.retain(|m| {
                tiles[Self::idx(m.col, m.row)].kind == TileKind::Ground
                    || action == EditAction::Place
            });
            self.recompute_ground_sprites();
        }
    }

    fn enter_play(&mut self) {
        self.mode = Mode::Play;
        self.keys = KeyState::default();
        self.reset_player_to_start();
        self.play_camera_left = (self.player.pos.x - 3.0 * TILE).clamp(0.0, LEVEL_W);
        self.score = 0;
        for money in &mut self.money {
            money.collected = false;
        }
    }

    fn enter_edit(&mut self) {
        self.mode = Mode::Edit;
        self.keys = KeyState::default();
        self.reset_player_to_start();
    }

    fn reset_player_to_start(&mut self) {
        self.player = PlayerState {
            pos: self.dino_start,
            vel: Vec2::ZERO,
            on_ground: false,
            anim_time: 0.0,
        };
    }

    fn solid_at(&self, col: isize, row: isize) -> bool {
        if !Self::in_bounds(col, row) {
            return false;
        }
        self.tile(col as usize, row as usize).kind == TileKind::Ground
    }

    fn aabb_collides(&self, pos: Vec2) -> bool {
        let left = ((pos.x - PLAYER_HALF_W) / TILE).floor() as isize;
        let right = ((pos.x + PLAYER_HALF_W - 0.1) / TILE).floor() as isize;
        let top = ((pos.y - PLAYER_HALF_H) / TILE).floor() as isize;
        let bottom = ((pos.y + PLAYER_HALF_H - 0.1) / TILE).floor() as isize;
        for row in top..=bottom {
            for col in left..=right {
                if self.solid_at(col, row) {
                    return true;
                }
            }
        }
        false
    }

    fn aabb_overlaps_kind(&self, pos: Vec2, kind: TileKind) -> bool {
        let left = ((pos.x - PLAYER_HALF_W) / TILE).floor() as isize;
        let right = ((pos.x + PLAYER_HALF_W - 0.1) / TILE).floor() as isize;
        let top = ((pos.y - PLAYER_HALF_H) / TILE).floor() as isize;
        let bottom = ((pos.y + PLAYER_HALF_H - 0.1) / TILE).floor() as isize;
        for row in top..=bottom {
            for col in left..=right {
                if Self::in_bounds(col, row) && self.tile(col as usize, row as usize).kind == kind {
                    return true;
                }
            }
        }
        false
    }

    fn fireball_overlaps_tile_material(&self, pos: Vec2) -> bool {
        let half = FIREBALL_SIZE * 0.5;
        let left = ((pos.x - half) / TILE).floor() as isize;
        let right = ((pos.x + half - 0.1) / TILE).floor() as isize;
        let top = ((pos.y - half) / TILE).floor() as isize;
        let bottom = ((pos.y + half - 0.1) / TILE).floor() as isize;
        for row in top..=bottom {
            for col in left..=right {
                if Self::in_bounds(col, row)
                    && self.tile(col as usize, row as usize).kind != TileKind::Empty
                {
                    return true;
                }
            }
        }
        false
    }

    fn update_player(&mut self) {
        if self.aabb_overlaps_kind(self.player.pos, TileKind::Lava) {
            self.enter_edit();
            return;
        }

        let in_water = self.aabb_overlaps_kind(self.player.pos, TileKind::Water);
        if in_water {
            if self.keys.right {
                self.player.vel.x =
                    (self.player.vel.x + WATER_HORIZONTAL_ACCEL * FIXED_DT).min(MAX_RUN_SPEED);
            }
            self.player.vel.x *= WATER_DRAG_X;
            self.player.vel.y = self.player.vel.y * WATER_DRAG_Y + WATER_GRAVITY * FIXED_DT;
            if self.keys.jump_pressed {
                self.player.vel.y =
                    (self.player.vel.y - WATER_JUMP_IMPULSE).max(-WATER_MAX_UP_SPEED);
            }
            self.player.vel.y = self
                .player
                .vel
                .y
                .clamp(-WATER_MAX_UP_SPEED, WATER_MAX_DOWN_SPEED);
            self.player.on_ground = false;
        } else {
            if self.keys.right {
                self.player.vel.x = (self.player.vel.x + MOVE_ACCEL * FIXED_DT).min(MAX_RUN_SPEED);
            } else {
                self.player.vel.x = (self.player.vel.x - MOVE_DECEL * FIXED_DT).max(0.0);
            }

            if self.keys.jump_pressed && self.player.on_ground {
                let jump_speed = if self.player.vel.x >= RUNNING_JUMP_THRESHOLD {
                    RUNNING_JUMP_SPEED
                } else {
                    STANDING_JUMP_SPEED
                };
                self.player.vel.y = -jump_speed;
                self.player.on_ground = false;
            }

            self.player.vel.y += GRAVITY * FIXED_DT;
        }

        self.keys.jump_pressed = false;

        let old_x = self.player.pos.x;
        self.player.pos.x += self.player.vel.x * FIXED_DT;
        self.player.pos.x = self
            .player
            .pos
            .x
            .clamp(PLAYER_HALF_W, LEVEL_W - PLAYER_HALF_W);
        if self.aabb_collides(self.player.pos) {
            self.player.pos.x = old_x;
            self.player.vel.x = 0.0;
        }

        self.player.pos.y += self.player.vel.y * FIXED_DT;
        self.player.on_ground = false;
        if self.aabb_collides(self.player.pos) {
            if self.player.vel.y > 0.0 {
                let ground_row = ((self.player.pos.y + PLAYER_HALF_H - 0.1) / TILE).floor();
                self.player.pos.y = ground_row * TILE - PLAYER_HALF_H;
                self.player.on_ground = true;
            } else if self.player.vel.y < 0.0 {
                let top_row = ((self.player.pos.y - PLAYER_HALF_H) / TILE).floor();
                self.player.pos.y = (top_row + 1.0) * TILE + PLAYER_HALF_H;
            }
            self.player.vel.y = 0.0;
        }

        let in_water_after_move = self.aabb_overlaps_kind(self.player.pos, TileKind::Water);
        if in_water_after_move {
            self.player.anim_time += FIXED_DT * WATER_ANIM_RATE;
        } else if self.player.vel.x > 10.0 && self.player.on_ground {
            self.player.anim_time += FIXED_DT;
        } else {
            self.player.anim_time = 0.0;
        }

        self.collect_money();

        if self.aabb_overlaps_kind(self.player.pos, TileKind::Lava) {
            self.enter_edit();
            return;
        }

        if self.player.pos.y > LEVEL_H + FALL_RETURN_MARGIN || self.reached_finish() {
            self.enter_edit();
        }
    }

    fn collect_money(&mut self) {
        for money in &mut self.money {
            if money.collected {
                continue;
            }
            let pos = Self::tile_center(money.col, money.row);
            if self.player.pos.distance(pos) <= MONEY_PICKUP_RADIUS {
                money.collected = true;
                self.score += money.value;
            }
        }
    }

    fn reached_finish(&self) -> bool {
        let gate_left = self.finish_col as f32 * TILE;
        let gate_right = gate_left + TILE * 2.0;
        let gate_bottom = (self.finish_row + 1) as f32 * TILE;
        let gate_top = gate_bottom - TILE * 3.0;
        self.player.pos.x + PLAYER_HALF_W > gate_left
            && self.player.pos.x - PLAYER_HALF_W < gate_right
            && self.player.pos.y + PLAYER_HALF_H > gate_top
            && self.player.pos.y - PLAYER_HALF_H < gate_bottom
    }

    fn update_hazards(&mut self) {
        for puff in &mut self.smoke {
            puff.age_frames += 1;
        }
        self.smoke.retain(|puff| puff.age_frames < SMOKE_FRAMES);

        let crater = self.crater_pos();
        let mut new_smoke = Vec::with_capacity(GARGLE_FIREBALL_COUNT + 1);
        for fireball in &mut self.gargle_fireballs {
            fireball.phase = (fireball.phase + 1.0 / fireball.period_frames).fract();
            let (pos, vel) = Self::gargle_position(crater, *fireball);
            new_smoke.push(SmokePuff {
                pos,
                age_frames: 0,
                rotation: Self::fireball_rotation(vel),
                size: FIREBALL_SIZE * 0.86,
            });
        }

        if self.eruption_timer_frames == 0 {
            self.start_eruption();
        } else {
            self.eruption_timer_frames -= 1;
        }

        let mut active_eruptions = std::mem::take(&mut self.eruptions);
        for mut eruption in active_eruptions.drain(..) {
            eruption.frame += 1;
            eruption.prev_pos = eruption.pos;
            let t = (eruption.frame as f32 / eruption.duration_frames as f32).clamp(0.0, 1.0);
            eruption.pos = Self::bezier2(eruption.start, eruption.apex, eruption.target, t);
            let vel = eruption.pos - eruption.prev_pos;
            new_smoke.push(SmokePuff {
                pos: eruption.pos,
                age_frames: 0,
                rotation: Self::fireball_rotation(vel),
                size: FIREBALL_SIZE,
            });

            let impacted_terrain = self.fireball_overlaps_tile_material(eruption.pos);
            let finished_path = eruption.frame >= eruption.duration_frames;
            if !impacted_terrain && !finished_path {
                if self.mode == Mode::Play
                    && self.player.pos.distance(eruption.pos) <= FIREBALL_HIT_RADIUS + PLAYER_HALF_W
                {
                    self.enter_edit();
                }
                self.eruptions.push(eruption);
            }
        }

        self.smoke.extend(new_smoke);
        if self.smoke.len() > 900 {
            let excess = self.smoke.len() - 900;
            self.smoke.drain(0..excess);
        }
    }

    fn edit_camera_for(viewport_w: f32, viewport_h: f32) -> CameraTransform {
        let scale_x = (viewport_w - EDIT_PADDING * 2.0) / LEVEL_W;
        let scale_y = (viewport_h - EDIT_PADDING * 2.0) / LEVEL_H;
        let scale = scale_x.min(scale_y);
        let visible_w = viewport_w / scale;
        let visible_h = viewport_h / scale;
        CameraTransform {
            left: -(visible_w - LEVEL_W) * 0.5,
            top: -(visible_h - LEVEL_H) * 0.5,
            scale,
        }
    }

    fn target_camera(&mut self) -> CameraTransform {
        match self.mode {
            Mode::Edit => Self::edit_camera_for(self.viewport_w, self.viewport_h),
            Mode::Play => {
                let scale = self.viewport_h.min(self.viewport_w) / (PLAY_VISIBLE_TILES * TILE);
                let visible_w = self.viewport_w / scale;
                let visible_h = self.viewport_h / scale;
                let desired_left = (self.player.pos.x - 3.0 * TILE).clamp(0.0, LEVEL_W - visible_w);
                self.play_camera_left = self.play_camera_left.max(desired_left);
                let max_left = (LEVEL_W - visible_w).max(0.0);
                self.play_camera_left = self.play_camera_left.clamp(0.0, max_left);
                let top = (self.player.pos.y - visible_h * 0.55)
                    .clamp(0.0, (LEVEL_H - visible_h).max(0.0));
                CameraTransform {
                    left: self.play_camera_left,
                    top,
                    scale,
                }
            }
        }
    }

    fn handle_input(&mut self, input: &InputQueue) {
        for event in input.iter() {
            match *event {
                InputEvent::PointerDown { x, y } => {
                    if self.mode != Mode::Edit {
                        continue;
                    }
                    let world = self.camera.screen_to_world(Vec2::new(x, y));
                    self.dragging = true;
                    self.last_cell = Self::cell_from_world(world);
                    if self.rectangle_mode {
                        self.rect_start = self.last_cell;
                    } else {
                        self.apply_piecewise_edit(world);
                    }
                }
                InputEvent::PointerMove { x, y } => {
                    if self.mode != Mode::Edit || !self.dragging {
                        continue;
                    }
                    let world = self.camera.screen_to_world(Vec2::new(x, y));
                    let cell = Self::cell_from_world(world);
                    if self.rectangle_mode {
                        self.last_cell = cell;
                    } else if cell != self.last_cell {
                        self.last_cell = cell;
                        self.apply_piecewise_edit(world);
                    }
                }
                InputEvent::PointerUp { x, y } => {
                    if self.mode == Mode::Edit && self.dragging && self.rectangle_mode {
                        let world = self.camera.screen_to_world(Vec2::new(x, y));
                        if let Some((col, row)) = Self::cell_from_world(world) {
                            self.apply_rectangle_edit(col, row);
                        }
                    }
                    self.dragging = false;
                    self.rect_start = None;
                    self.last_cell = None;
                }
                InputEvent::KeyDown { key_code } => match key_code {
                    39 | 68 => self.keys.right = true,
                    32 | 38 | 87 => {
                        if !self.keys.jump {
                            self.keys.jump_pressed = true;
                        }
                        self.keys.jump = true;
                    }
                    _ => {}
                },
                InputEvent::KeyUp { key_code } => match key_code {
                    39 | 68 => self.keys.right = false,
                    32 | 38 | 87 => self.keys.jump = false,
                    _ => {}
                },
                InputEvent::Custom { kind, a, b, .. } => match kind {
                    CUSTOM_SET_TOOL => self.edit_tool = EditTool::from_code(a),
                    CUSTOM_SET_ACTION => self.edit_action = EditAction::from_code(a),
                    CUSTOM_SET_RECTANGLE => self.rectangle_mode = a >= 0.5,
                    CUSTOM_SET_DINO_COLOR => self.dino_color = DinoColor::from_code(a),
                    CUSTOM_PLAY => self.enter_play(),
                    CUSTOM_EDIT => self.enter_edit(),
                    CUSTOM_RESET_LEVEL => {
                        if self.mode == Mode::Edit {
                            self.reset_to_seed_level();
                        }
                    }
                    CUSTOM_RESIZE => {
                        self.viewport_w = a.max(1.0);
                        self.viewport_h = b.max(1.0);
                    }
                    _ => {}
                },
            }
        }
    }

    fn emit_state_events(&mut self, ctx: &mut EngineContext) {
        if self.last_emitted_mode != self.mode {
            self.last_emitted_mode = self.mode;
            ctx.emit_event(GameEvent {
                kind: EVENT_MODE,
                a: if self.mode == Mode::Play { 1.0 } else { 0.0 },
                b: 0.0,
                c: 0.0,
            });
        }
        if self.last_emitted_score != self.score {
            self.last_emitted_score = self.score;
            ctx.emit_event(GameEvent {
                kind: EVENT_SCORE,
                a: self.score as f32,
                b: 0.0,
                c: 0.0,
            });
        }
    }

    fn sync_scene(&self, ctx: &mut EngineContext) {
        ctx.scene.clear();
        self.draw_background_and_grid(ctx);
        self.spawn_volcano(ctx);
        self.spawn_tiles(ctx);
        self.spawn_finish_back(ctx);
        self.spawn_smoke(ctx);
        self.spawn_money(ctx);
        self.spawn_dino(ctx);
        self.spawn_fireballs(ctx);
        self.spawn_finish_front(ctx);
        self.draw_rectangle_preview(ctx);
    }

    fn draw_background_and_grid(&self, ctx: &mut EngineContext) {
        let p0 = self.camera.world_to_screen(Vec2::ZERO);
        let p1 = self.camera.world_to_screen(Vec2::new(LEVEL_W, LEVEL_H));
        ctx.vectors.stroke_rect(
            p0,
            p1.x - p0.x,
            p1.y - p0.y,
            3.0,
            VectorColor::new(0.35, 0.65, 0.95, 0.65),
        );

        if self.mode == Mode::Edit {
            let grid_color = VectorColor::new(0.35, 0.55, 0.7, 0.22);
            for col in 0..=LEVEL_COLS {
                let x = col as f32 * TILE;
                let a = self.camera.world_to_screen(Vec2::new(x, 0.0));
                let b = self.camera.world_to_screen(Vec2::new(x, LEVEL_H));
                ctx.vectors.stroke_polyline(&[a, b], 1.0, grid_color);
            }
            for row in 0..=LEVEL_ROWS {
                let y = row as f32 * TILE;
                let a = self.camera.world_to_screen(Vec2::new(0.0, y));
                let b = self.camera.world_to_screen(Vec2::new(LEVEL_W, y));
                ctx.vectors.stroke_polyline(&[a, b], 1.0, grid_color);
            }
        }
    }

    fn draw_rectangle_preview(&self, ctx: &mut EngineContext) {
        if self.mode != Mode::Edit || !self.rectangle_mode || !self.dragging {
            return;
        }
        let (Some((start_col, start_row)), Some((end_col, end_row))) =
            (self.rect_start, self.last_cell)
        else {
            return;
        };

        let min_col = start_col.min(end_col);
        let max_col = start_col.max(end_col);
        let min_row = start_row.min(end_row);
        let max_row = start_row.max(end_row);
        let world_top_left = Vec2::new(min_col as f32 * TILE, min_row as f32 * TILE);
        let world_bottom_right =
            Vec2::new((max_col + 1) as f32 * TILE, (max_row + 1) as f32 * TILE);
        let screen_top_left = self.camera.world_to_screen(world_top_left);
        let screen_bottom_right = self.camera.world_to_screen(world_bottom_right);
        let width = screen_bottom_right.x - screen_top_left.x;
        let height = screen_bottom_right.y - screen_top_left.y;

        let (fill, stroke) = match self.edit_action {
            EditAction::Place => (
                VectorColor::new(0.4, 1.0, 0.55, 0.18),
                VectorColor::new(0.55, 1.0, 0.66, 0.95),
            ),
            EditAction::Erase => (
                VectorColor::new(1.0, 0.25, 0.22, 0.16),
                VectorColor::new(1.0, 0.42, 0.36, 0.95),
            ),
        };

        ctx.vectors.fill_rect(screen_top_left, width, height, fill);
        ctx.vectors
            .stroke_rect(screen_top_left, width, height, 4.0, stroke);
    }

    fn spawn_tiles(&self, ctx: &mut EngineContext) {
        for row in 0..LEVEL_ROWS {
            for col in 0..LEVEL_COLS {
                let tile = self.tile(col, row);
                if tile.kind == TileKind::Empty {
                    continue;
                }
                let center = Self::tile_center(col, row);
                let sprite_name = match tile.kind {
                    TileKind::Empty => continue,
                    TileKind::Ground => tile.sprite.name(),
                    TileKind::Lava => "lava",
                    TileKind::Water => "water",
                };
                self.spawn_sprite(ctx, sprite_name, center, TILE, RenderLayer::Terrain, 1.0);
            }
        }
    }

    fn spawn_volcano(&self, ctx: &mut EngineContext) {
        self.spawn_sprite(
            ctx,
            "vulcan",
            self.volcano_center(),
            VOLCANO_SIZE,
            RenderLayer::Background,
            1.0,
        );
    }

    fn spawn_smoke(&self, ctx: &mut EngineContext) {
        for puff in &self.smoke {
            let alpha = 1.0 - puff.age_frames as f32 / SMOKE_FRAMES as f32;
            self.spawn_sprite_rotated(
                ctx,
                "smoke",
                puff.pos,
                puff.size,
                puff.rotation,
                RenderLayer::Terrain,
                alpha.clamp(0.0, 1.0),
            );
        }
    }

    fn spawn_money(&self, ctx: &mut EngineContext) {
        for money in &self.money {
            if money.collected {
                continue;
            }
            let sprite = if money.value == 10 {
                "ban-x10"
            } else {
                "ban-x1"
            };
            self.spawn_sprite(
                ctx,
                sprite,
                Self::tile_center(money.col, money.row),
                TILE * 0.82,
                RenderLayer::Objects,
                1.0,
            );
        }
    }

    fn spawn_dino(&self, ctx: &mut EngineContext) {
        let player_in_water =
            self.mode == Mode::Play && self.aabb_overlaps_kind(self.player.pos, TileKind::Water);
        let frame = if self.mode == Mode::Play
            && ((self.player.vel.x > 10.0 && self.player.on_ground) || player_in_water)
        {
            ((self.player.anim_time / ANIM_FRAME_TIME).floor() as usize % 4) + 1
        } else {
            1
        };
        let sprite = self.dino_color.sprite_name(frame);
        let pos = if self.mode == Mode::Play {
            self.player.pos
        } else {
            self.dino_start
        };
        self.spawn_sprite(ctx, &sprite, pos, TILE, RenderLayer::Objects, 1.0);

        if self.mode == Mode::Edit {
            let s = self.camera.world_to_screen(pos);
            let half = PLAYER_HALF_H * self.camera.scale;
            ctx.vectors
                .stroke_circle(s, half, 3.0, VectorColor::new(0.4, 1.0, 0.55, 0.95));
        }
    }

    fn finish_world_positions(&self) -> (Vec2, Vec2) {
        let left_center = Vec2::new(
            self.finish_col as f32 * TILE + TILE * 0.5,
            (self.finish_row + 1) as f32 * TILE - TILE * 1.5,
        );
        let right_center = left_center + Vec2::new(TILE, 0.0);
        (left_center, right_center)
    }

    fn spawn_finish_back(&self, ctx: &mut EngineContext) {
        let (left, _) = self.finish_world_positions();
        self.spawn_sprite(
            ctx,
            "finish-spate-square",
            left,
            TILE * 3.0,
            RenderLayer::Terrain,
            1.0,
        );
    }

    fn spawn_finish_front(&self, ctx: &mut EngineContext) {
        let (_, right) = self.finish_world_positions();
        self.spawn_sprite(
            ctx,
            "finish-fata-square",
            right,
            TILE * 3.0,
            RenderLayer::Foreground,
            1.0,
        );
    }

    fn spawn_fireballs(&self, ctx: &mut EngineContext) {
        let crater = self.crater_pos();
        for fireball in self.gargle_fireballs {
            let (pos, vel) = Self::gargle_position(crater, fireball);
            self.spawn_fireball(ctx, pos, FIREBALL_SIZE, Self::fireball_rotation(vel));
        }
        for eruption in &self.eruptions {
            let vel = eruption.pos - eruption.prev_pos;
            self.spawn_fireball(
                ctx,
                eruption.pos,
                FIREBALL_SIZE * 1.14,
                Self::fireball_rotation(vel),
            );
        }
    }

    fn spawn_fireball(
        &self,
        ctx: &mut EngineContext,
        world_pos: Vec2,
        world_size: f32,
        rotation: f32,
    ) {
        self.spawn_sprite_rotated_with_blend(
            ctx,
            "fireball",
            world_pos,
            world_size,
            rotation,
            RenderLayer::Objects,
            FIREBALL_HDR_ALPHA,
            BlendMode::Additive,
        );
    }

    fn spawn_sprite(
        &self,
        ctx: &mut EngineContext,
        name: &str,
        world_pos: Vec2,
        world_size: f32,
        layer: RenderLayer,
        alpha: f32,
    ) {
        self.spawn_sprite_rotated(ctx, name, world_pos, world_size, 0.0, layer, alpha);
    }

    fn spawn_sprite_rotated(
        &self,
        ctx: &mut EngineContext,
        name: &str,
        world_pos: Vec2,
        world_size: f32,
        rotation: f32,
        layer: RenderLayer,
        alpha: f32,
    ) {
        self.spawn_sprite_rotated_with_blend(
            ctx,
            name,
            world_pos,
            world_size,
            rotation,
            layer,
            alpha,
            BlendMode::Alpha,
        );
    }

    fn spawn_sprite_rotated_with_blend(
        &self,
        ctx: &mut EngineContext,
        name: &str,
        world_pos: Vec2,
        world_size: f32,
        rotation: f32,
        layer: RenderLayer,
        alpha: f32,
        blend: BlendMode,
    ) {
        let screen_pos = self.camera.world_to_screen(world_pos);
        let screen_size = world_size * self.camera.scale;
        if screen_pos.x + screen_size < -64.0
            || screen_pos.x - screen_size > self.viewport_w + 64.0
            || screen_pos.y + screen_size < -64.0
            || screen_pos.y - screen_size > self.viewport_h + 64.0
        {
            return;
        }

        let Some(mut sprite) = ctx.sprite(name) else {
            return;
        };
        sprite.alpha = alpha;
        sprite.blend = blend;
        let id = ctx.next_id();
        ctx.scene.spawn(
            Entity::new(id)
                .with_tag(name)
                .with_layer(layer)
                .with_pos(screen_pos)
                .with_rotation(rotation)
                .with_scale(Vec2::splat(screen_size))
                .with_sprite(sprite),
        );
    }
}

impl Game for DinoLevelEditor {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: FIXED_DT,
            world_width: VIEW_W,
            world_height: VIEW_H,
            max_entities: 4096,
            max_instances: 4096,
            max_vector_vertices: 20_000,
            max_layer_batches: 160,
            ..Default::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        ctx.lights.set_ambient(1.0, 1.0, 1.0);
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        self.handle_input(input);
        if self.mode == Mode::Play {
            self.update_player();
        }
        self.update_hazards();

        let target = self.target_camera();
        self.camera.lerp_toward(target);
        self.sync_scene(ctx);
        self.emit_state_events(ctx);
    }
}
