use glam::Vec2;
use zap_engine::*;

use crate::animation::{AnimationState, FallAnim, RotateAnim};
use crate::board::*;

// Board layout constants
const BOARD_WIDTH: usize = 8;
const BOARD_HEIGHT: usize = 8;
const TILE_SIZE: f32 = 50.0;
const WORLD_WIDTH: f32 = 800.0;
const WORLD_HEIGHT: f32 = 600.0;
const GRID_OFFSET_X: f32 = (WORLD_WIDTH - BOARD_WIDTH as f32 * TILE_SIZE) / 2.0;
const GRID_OFFSET_Y: f32 = (WORLD_HEIGHT - BOARD_HEIGHT as f32 * TILE_SIZE) / 2.0;
const MISSING_LINKS_PERCENT: usize = 3;
const SEED: u64 = 42;

// Freeze duration for the zap display (seconds)
const FREEZE_ZAP_DURATION: f32 = 2.0;

// Game event kinds (Rust → React)
const EVENT_SCORE: f32 = 1.0;

// Custom event kinds (React → Rust)
const CUSTOM_NEW_GAME: u32 = 1;

/// Game state machine phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GamePhase {
    WaitingForInput,
    RotatingTile,
    FallingTiles,
    FreezeDuringZap,
}

pub struct ZapZapMini {
    board: GameBoard,
    anims: AnimationState,
    phase: GamePhase,
    score: u32,
    /// Whether we have a pending connection check after rotation completes
    pending_check: bool,
}

impl ZapZapMini {
    pub fn new() -> Self {
        Self {
            board: GameBoard::new(BOARD_WIDTH, BOARD_HEIGHT, MISSING_LINKS_PERCENT, SEED),
            anims: AnimationState::new(),
            phase: GamePhase::WaitingForInput,
            score: 0,
            pending_check: false,
        }
    }

    /// Convert world coordinates to grid cell (x, y).
    fn world_to_grid(wx: f32, wy: f32) -> Option<(usize, usize)> {
        let gx = ((wx - GRID_OFFSET_X) / TILE_SIZE).floor() as i32;
        let gy = ((wy - GRID_OFFSET_Y) / TILE_SIZE).floor() as i32;
        if gx >= 0 && gx < BOARD_WIDTH as i32 && gy >= 0 && gy < BOARD_HEIGHT as i32 {
            Some((gx as usize, gy as usize))
        } else {
            None
        }
    }

    /// Get the world-space center position of a grid cell.
    fn tile_center(x: usize, y: usize) -> Vec2 {
        Vec2::new(
            GRID_OFFSET_X + x as f32 * TILE_SIZE + TILE_SIZE * 0.5,
            GRID_OFFSET_Y + y as f32 * TILE_SIZE + TILE_SIZE * 0.5,
        )
    }

    /// Handle a tap on a tile: rotate it and start animation.
    fn tap_tile(&mut self, x: usize, y: usize) {
        if let Some(tile) = self.board.grid.get_mut(x, y) {
            tile.rotate();
            self.anims.rotate_anims.push(RotateAnim::new(x, y, 1));
            self.phase = GamePhase::RotatingTile;
            self.pending_check = true;
        }
    }

    /// Build arcs for all marked tiles using the engine's effects system.
    fn build_arcs(&self, effects: &mut EffectsState) {
        let left_pin_x = GRID_OFFSET_X - TILE_SIZE + TILE_SIZE * 0.5;
        let right_pin_x = GRID_OFFSET_X + BOARD_WIDTH as f32 * TILE_SIZE + TILE_SIZE * 0.5;

        let configs: [(Marking, SegmentColor, u32, f32); 3] = [
            (Marking::Left, SegmentColor::Indigo, 4, 4.0),
            (Marking::Right, SegmentColor::Orange, 4, 4.0),
            (Marking::Ok, SegmentColor::SkyBlue, 3, 8.0),
        ];

        for &(marker, color, po2, width) in &configs {
            for y in 0..BOARD_HEIGHT {
                // Left pin arcs
                if self.board.get_marking(0, y) == marker {
                    if let Some(tile) = self.board.grid.get(0, y) {
                        if tile.has_connection(Direction::LEFT) {
                            let pin = [left_pin_x, GRID_OFFSET_Y + y as f32 * TILE_SIZE + TILE_SIZE * 0.5];
                            let center = Self::tile_center(0, y);
                            let pin_color = if marker == Marking::Ok { SegmentColor::Red } else { color };
                            effects.add_arc(pin, [center.x, center.y], width, pin_color, po2);
                        }
                    }
                }

                // Right pin arcs
                if self.board.get_marking(BOARD_WIDTH - 1, y) == marker {
                    if let Some(tile) = self.board.grid.get(BOARD_WIDTH - 1, y) {
                        if tile.has_connection(Direction::RIGHT) {
                            let center = Self::tile_center(BOARD_WIDTH - 1, y);
                            let pin = [right_pin_x, GRID_OFFSET_Y + y as f32 * TILE_SIZE + TILE_SIZE * 0.5];
                            let pin_color = if marker == Marking::Ok { SegmentColor::Red } else { color };
                            effects.add_arc([center.x, center.y], pin, width, pin_color, po2);
                        }
                    }
                }

                // Internal tile-to-tile arcs
                for x in 0..BOARD_WIDTH {
                    if self.board.get_marking(x, y) != marker {
                        continue;
                    }
                    if let Some(tile) = self.board.grid.get(x, y) {
                        // Horizontal: right
                        if tile.has_connection(Direction::RIGHT) && x + 1 < BOARD_WIDTH {
                            if let Some(right) = self.board.grid.get(x + 1, y) {
                                if right.has_connection(Direction::LEFT)
                                    && self.board.get_marking(x + 1, y) == marker
                                {
                                    let a = Self::tile_center(x, y);
                                    let b = Self::tile_center(x + 1, y);
                                    effects.add_arc([a.x, a.y], [b.x, b.y], width, color, po2);
                                }
                            }
                        }
                        // Vertical: down
                        if tile.has_connection(Direction::DOWN) && y + 1 < BOARD_HEIGHT {
                            if let Some(down) = self.board.grid.get(x, y + 1) {
                                if down.has_connection(Direction::UP)
                                    && self.board.get_marking(x, y + 1) == marker
                                {
                                    let a = Self::tile_center(x, y);
                                    let b = Self::tile_center(x, y + 1);
                                    effects.add_arc([a.x, a.y], [b.x, b.y], width, color, po2);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Spawn dynamic point lights at marked tile positions for dramatic lighting.
    fn build_lights(&self, lights: &mut LightState) {
        lights.clear();
        lights.set_ambient(0.15, 0.15, 0.2);

        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                let marking = self.board.get_marking(x, y);
                let center = Self::tile_center(x, y);

                match marking {
                    Marking::Ok => {
                        // Bright blue-white light with wiggle for arc-light flicker
                        let wiggle_x = (self.board.rng.state.wrapping_mul(x as u64 + 1).wrapping_add(y as u64) % 600) as f32 / 100.0 - 3.0;
                        let wiggle_y = (self.board.rng.state.wrapping_mul(y as u64 + 1).wrapping_add(x as u64) % 600) as f32 / 100.0 - 3.0;
                        lights.add(PointLight::new(
                            Vec2::new(center.x + wiggle_x, center.y + wiggle_y),
                            [0.4, 0.7, 1.0],
                            1.5,
                            120.0,
                        ));
                    }
                    Marking::Left => {
                        lights.add(PointLight::new(center, [0.3, 0.2, 0.8], 0.8, 80.0));
                    }
                    Marking::Right => {
                        lights.add(PointLight::new(center, [0.8, 0.5, 0.2], 0.8, 80.0));
                    }
                    _ => {}
                }
            }
        }
    }

    /// Rebuild all tile entity sprites + positions based on current board state.
    fn sync_entities(&self, ctx: &mut EngineContext) {
        // Clear scene — we rebuild every frame (simple approach for 64+ entities)
        ctx.scene.clear();

        // Background entity (dark board area)
        let bg_id = ctx.next_id();
        ctx.scene.spawn(
            Entity::new(bg_id)
                .with_tag("bg")
                .with_pos(Vec2::new(WORLD_WIDTH / 2.0, WORLD_HEIGHT / 2.0))
                .with_scale(Vec2::new(WORLD_WIDTH, WORLD_HEIGHT))
                .with_layer(RenderLayer::Background)
                .with_sprite(SpriteComponent {
                    atlas: AtlasId(0),
                    col: 0.0,
                    row: 0.0, // row 0 — background tile
                    cell_span: 1.0,
                    alpha: 1.0,
                    blend: BlendMode::Alpha,
                }),
        );

        // Left pins
        for y in 0..BOARD_HEIGHT {
            let pin_id = ctx.next_id();
            let pin_x = GRID_OFFSET_X - TILE_SIZE * 0.5;
            let pin_y = GRID_OFFSET_Y + y as f32 * TILE_SIZE + TILE_SIZE * 0.5;
            ctx.scene.spawn(
                Entity::new(pin_id)
                    .with_pos(Vec2::new(pin_x, pin_y))
                    .with_scale(Vec2::splat(TILE_SIZE))
                    .with_layer(RenderLayer::Terrain)
                    .with_sprite(SpriteComponent {
                        atlas: AtlasId(0),
                        col: ATLAS_COL_LEFT_PIN,
                        row: ATLAS_ROW_PINS,
                        cell_span: 1.0,
                        alpha: 1.0,
                        blend: BlendMode::Alpha,
                    }),
            );
        }

        // Right pins
        for y in 0..BOARD_HEIGHT {
            let pin_id = ctx.next_id();
            let pin_x = GRID_OFFSET_X + BOARD_WIDTH as f32 * TILE_SIZE + TILE_SIZE * 0.5;
            let pin_y = GRID_OFFSET_Y + y as f32 * TILE_SIZE + TILE_SIZE * 0.5;
            ctx.scene.spawn(
                Entity::new(pin_id)
                    .with_pos(Vec2::new(pin_x, pin_y))
                    .with_scale(Vec2::splat(TILE_SIZE))
                    .with_layer(RenderLayer::Terrain)
                    .with_sprite(SpriteComponent {
                        atlas: AtlasId(0),
                        col: ATLAS_COL_RIGHT_PIN,
                        row: ATLAS_ROW_PINS,
                        cell_span: 1.0,
                        alpha: 1.0,
                        blend: BlendMode::Alpha,
                    }),
            );
        }

        // Tile entities
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                if let Some(tile) = self.board.grid.get(x, y) {
                    let tile_id = ctx.next_id();
                    let mut center = Self::tile_center(x, y);
                    let mut rotation = 0.0f32;

                    // Apply animation overrides
                    if let Some(rot) = self.anims.get_rotation(x, y) {
                        rotation = rot;
                    }
                    if let Some(fall_y) = self.anims.get_fall_y(x, y) {
                        center.y = fall_y;
                    }

                    let atlas_col = GRID_CODEP[tile.connections as usize] as f32;
                    let layer = if self.anims.get_fall_y(x, y).is_some() {
                        RenderLayer::Objects
                    } else {
                        RenderLayer::Terrain
                    };

                    ctx.scene.spawn(
                        Entity::new(tile_id)
                            .with_pos(center)
                            .with_rotation(rotation)
                            .with_scale(Vec2::splat(TILE_SIZE))
                            .with_layer(layer)
                            .with_sprite(SpriteComponent {
                                atlas: AtlasId(0),
                                col: atlas_col,
                                row: ATLAS_ROW_NORMAL,
                                cell_span: 1.0,
                                alpha: 1.0,
                                blend: BlendMode::Alpha,
                            }),
                    );
                }
            }
        }
    }

    fn new_game(&mut self) {
        self.board = GameBoard::new(BOARD_WIDTH, BOARD_HEIGHT, MISSING_LINKS_PERCENT, self.board.rng.next_u64());
        self.anims.clear();
        self.phase = GamePhase::WaitingForInput;
        self.score = 0;
        self.pending_check = false;
    }
}

impl Game for ZapZapMini {
    fn config(&self) -> GameConfig {
        GameConfig {
            world_width: WORLD_WIDTH,
            world_height: WORLD_HEIGHT,
            max_instances: 256,
            max_effects_vertices: 32768,
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        // Run initial connection check so markings are visible from the start
        self.board.check_connections();
        self.sync_entities(ctx);
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        let dt = 1.0 / 60.0f32;

        // Handle custom events (New Game)
        for event in input.iter() {
            if let InputEvent::Custom { kind, .. } = event {
                if *kind == CUSTOM_NEW_GAME {
                    self.new_game();
                    self.board.check_connections();
                    ctx.emit_event(GameEvent { kind: EVENT_SCORE, a: 0.0, b: 0.0, c: 0.0 });
                }
            }
        }

        match self.phase {
            GamePhase::WaitingForInput => {
                // Handle pointer down → rotate tile
                for event in input.iter() {
                    if let InputEvent::PointerDown { x, y } = event {
                        if let Some((gx, gy)) = Self::world_to_grid(*x, *y) {
                            self.tap_tile(gx, gy);
                            break;
                        }
                    }
                }
            }

            GamePhase::RotatingTile => {
                self.anims.tick_rotations(dt);
                if !self.anims.has_rotate_anims() {
                    // Rotation complete — check for connections
                    if self.pending_check {
                        self.pending_check = false;
                        let connected = self.board.check_connections();
                        if connected > 0 {
                            // Score the connected tiles
                            let zapped = self.board.count_ok_tiles() as u32;
                            self.score += zapped;
                            ctx.emit_event(GameEvent {
                                kind: EVENT_SCORE,
                                a: self.score as f32,
                                b: zapped as f32,
                                c: 0.0,
                            });

                            // Spawn particles at ok-marked tiles
                            for py in 0..BOARD_HEIGHT {
                                for px in 0..BOARD_WIDTH {
                                    if self.board.get_marking(px, py) == Marking::Ok {
                                        let center = Self::tile_center(px, py);
                                        ctx.effects.spawn_particles(
                                            [center.x, center.y],
                                            10,
                                            10.0,
                                            4.0,
                                            2.0,
                                        );
                                    }
                                }
                            }

                            // Freeze to show arcs before removing tiles
                            self.anims.freeze_timer = FREEZE_ZAP_DURATION;
                            self.phase = GamePhase::FreezeDuringZap;
                        } else {
                            self.phase = GamePhase::WaitingForInput;
                        }
                    } else {
                        self.phase = GamePhase::WaitingForInput;
                    }
                }
            }

            GamePhase::FreezeDuringZap => {
                if self.anims.tick_freeze(dt) {
                    // Freeze ended — remove tiles and start gravity
                    let _falls = self.board.remove_and_shift();

                    // Create fall animations for all tiles (simple: everything shifts)
                    for x in 0..BOARD_WIDTH {
                        for y in 0..BOARD_HEIGHT {
                            // Animate from one tile above to current position
                            let target = Self::tile_center(x, y);
                            let start_y = target.y - TILE_SIZE;
                            if start_y >= GRID_OFFSET_Y - TILE_SIZE {
                                self.anims.fall_anims.push(
                                    FallAnim::new(x, y, start_y, target.y),
                                );
                            }
                        }
                    }

                    self.phase = GamePhase::FallingTiles;
                    // Re-check after removal
                    self.board.check_connections();
                }
            }

            GamePhase::FallingTiles => {
                self.anims.tick_falls();
                if !self.anims.has_fall_anims() {
                    // Falling complete — check for new connections
                    self.board.check_connections();
                    self.phase = GamePhase::WaitingForInput;
                }
            }
        }

        // Always rebuild arcs for any marked tiles (arcs twitch naturally each frame)
        self.build_arcs(&mut ctx.effects);

        // Rebuild lights if we have marked tiles
        self.build_lights(&mut ctx.lights);

        // Sync entity state
        self.sync_entities(ctx);
    }
}
