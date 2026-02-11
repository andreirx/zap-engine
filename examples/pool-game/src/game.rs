//! Pool game - 2D billiards with Rapier2D physics and SDF rendering.

use glam::Vec2;
use zap_engine::api::game::GameConfig;
use zap_engine::api::types::EntityId;
use zap_engine::components::entity::Entity;
use zap_engine::components::layer::RenderLayer;
use zap_engine::components::mesh::MeshComponent;
use zap_engine::input::queue::{InputEvent, InputQueue};
use zap_engine::{BodyDesc, ColliderDesc, ColliderMaterial};
use zap_engine::VectorColor;
use zap_engine::{EngineContext, Game, GameEvent};

use crate::balls::{rack_positions, BallType, BALLS};

// World dimensions (matches pool table aspect ratio ~2:1)
const WORLD_W: f32 = 1000.0;
const WORLD_H: f32 = 500.0;

// Table visual and physics
const RAIL_WIDTH: f32 = 35.0;  // Visual rail width
const CUSHION: f32 = 35.0;  // Physics boundary = rail width (ball EDGE touches rail)
const POCKET_GAP: f32 = 40.0;  // Gap in cushions at pockets

// Debug visualization
const DEBUG_DRAW: bool = false;

// Ball properties
const BALL_RADIUS: f32 = 12.0;

// Pocket properties
const POCKET_RADIUS: f32 = 28.0;  // Larger pockets
const CORNER_POCKET_INSET: f32 = 22.0;
const SIDE_POCKET_INSET: f32 = 18.0;

// Cue ball starting position (left side of table)
const CUE_START_X: f32 = 250.0;
const CUE_START_Y: f32 = WORLD_H / 2.0;

// Rack apex position (right side of table)
const RACK_X: f32 = 700.0;
const RACK_Y: f32 = WORLD_H / 2.0;

// Physics parameters
const LINEAR_DAMPING: f32 = 0.5;   // Felt friction (lower = slides more)
const ANGULAR_DAMPING: f32 = 0.3;
const RESTITUTION: f32 = 0.95;     // Bouncy ball-to-ball
const FRICTION: f32 = 0.2;
const DENSITY: f32 = 0.01;  // Very low density so impulse = velocity

// Aiming - use velocity directly, not impulse
const MAX_SHOT_SPEED: f32 = 2400.0;  // 4x power
const SHOT_SCALE: f32 = 6.0;         // 4x power

/// Custom event kinds from React UI
mod events {
    pub const RESET: u32 = 1;
}

/// Game event kinds to React
mod game_events {
    pub const BALLS_REMAINING: f32 = 1.0;
}

/// Game state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameState {
    Aiming,
    BallsMoving,
}

/// Ball entity tracking
struct BallEntity {
    entity_id: EntityId,
    ball_number: u8,
    pocketed: bool,
}

pub struct PoolGame {
    state: GameState,
    aiming: bool,
    aim_start: Vec2,
    aim_current: Vec2,
    cue_ball_id: Option<EntityId>,
    balls: Vec<BallEntity>,
    table_id: Option<EntityId>,
}

impl PoolGame {
    pub fn new() -> Self {
        Self {
            state: GameState::Aiming,
            aiming: false,
            aim_start: Vec2::ZERO,
            aim_current: Vec2::ZERO,
            cue_ball_id: None,
            balls: Vec::with_capacity(16),
            table_id: None,
        }
    }

    /// Ensure felt sprites exist (2x2 grid for finer grain)
    fn ensure_felt(&mut self, ctx: &mut EngineContext) {
        if self.table_id.is_some() {
            return;
        }

        let sprite = match ctx.sprite("felt") {
            Some(s) => s,
            None => return,  // Manifest not loaded yet, try again next frame
        };

        // Tile 2 felt sprites for finer grain texture
        let tile_size = WORLD_W / 2.0;  // Each tile covers half the width
        let positions = [
            Vec2::new(tile_size * 0.5, WORLD_H * 0.5),
            Vec2::new(tile_size * 1.5, WORLD_H * 0.5),
        ];

        let mut first_id = None;
        for (i, pos) in positions.iter().enumerate() {
            let id = ctx.next_id();
            let entity = Entity::new(id)
                .with_tag(&format!("felt_{}", i))
                .with_pos(*pos)
                .with_scale(Vec2::splat(tile_size))
                .with_layer(RenderLayer::Background)
                .with_sprite(sprite.clone());
            ctx.scene.spawn(entity);

            if first_id.is_none() {
                first_id = Some(id);
            }
        }
        self.table_id = first_id;
        log::info!("Felt sprites spawned");
    }

    /// Build table cushion walls with gaps for pockets
    fn build_cushions(ctx: &mut EngineContext) {
        let wall_material = ColliderMaterial {
            restitution: 0.7,  // Cushion bounce
            friction: 0.2,
            density: 1.0,
        };

        // Play area boundaries
        let left = CUSHION;
        let right = WORLD_W - CUSHION;
        let top = CUSHION;
        let bottom = WORLD_H - CUSHION;
        let mid_x = WORLD_W / 2.0;
        let corner_gap = POCKET_GAP;  // Gap at corners for corner pockets
        let side_gap = POCKET_GAP;    // Gap at middle for side pockets

        // Top cushion - split into left and right segments (gap at corners and middle)
        // Left segment: from corner gap to side pocket gap
        let seg_len = mid_x - left - corner_gap - side_gap / 2.0;
        if seg_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: seg_len / 2.0,
                half_height: CUSHION / 2.0,
            })
            .with_position(Vec2::new(left + corner_gap + seg_len / 2.0, CUSHION / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }
        // Right segment
        if seg_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: seg_len / 2.0,
                half_height: CUSHION / 2.0,
            })
            .with_position(Vec2::new(right - corner_gap - seg_len / 2.0, CUSHION / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }

        // Bottom cushion - same pattern
        if seg_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: seg_len / 2.0,
                half_height: CUSHION / 2.0,
            })
            .with_position(Vec2::new(left + corner_gap + seg_len / 2.0, WORLD_H - CUSHION / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }
        if seg_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: seg_len / 2.0,
                half_height: CUSHION / 2.0,
            })
            .with_position(Vec2::new(right - corner_gap - seg_len / 2.0, WORLD_H - CUSHION / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }

        // Left cushion - one piece with corner gaps at top and bottom
        let side_len = bottom - top - 2.0 * corner_gap;
        if side_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: CUSHION / 2.0,
                half_height: side_len / 2.0,
            })
            .with_position(Vec2::new(CUSHION / 2.0, WORLD_H / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }

        // Right cushion - same pattern
        if side_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: CUSHION / 2.0,
                half_height: side_len / 2.0,
            })
            .with_position(Vec2::new(WORLD_W - CUSHION / 2.0, WORLD_H / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }
    }

    /// Draw table frame (rails, pockets with cuts) with vectors
    fn draw_table_frame(&self, ctx: &mut EngineContext) {
        // Very dark brown - no HDR strain
        let rail_color = VectorColor::new(0.12, 0.06, 0.02, 1.0);
        let pocket_color = VectorColor::new(0.0, 0.0, 0.0, 1.0);
        let cut_color = VectorColor::new(0.02, 0.02, 0.02, 1.0);  // Dark for pocket cuts

        // Draw rails
        ctx.vectors.fill_rect(Vec2::ZERO, WORLD_W, RAIL_WIDTH, rail_color);  // Top
        ctx.vectors.fill_rect(Vec2::new(0.0, WORLD_H - RAIL_WIDTH), WORLD_W, RAIL_WIDTH, rail_color);  // Bottom
        ctx.vectors.fill_rect(Vec2::new(0.0, RAIL_WIDTH), RAIL_WIDTH, WORLD_H - 2.0 * RAIL_WIDTH, rail_color);  // Left
        ctx.vectors.fill_rect(Vec2::new(WORLD_W - RAIL_WIDTH, RAIL_WIDTH), RAIL_WIDTH, WORLD_H - 2.0 * RAIL_WIDTH, rail_color);  // Right

        // Draw pockets with cuts into the rails
        let pockets = Self::pocket_positions();
        let cut_size = POCKET_RADIUS + 10.0;

        for (i, pos) in pockets.iter().enumerate() {
            // Main pocket hole
            ctx.vectors.fill_circle(*pos, POCKET_RADIUS + 8.0, pocket_color);

            // Draw cuts/notches leading into pocket
            // Corner pockets (0-3) get diagonal cuts, side pockets (4-5) get vertical cuts
            if i < 4 {
                // Corner pocket - draw a larger area to cut into the corner
                ctx.vectors.fill_circle(*pos, cut_size, cut_color);
            } else {
                // Side pocket - rectangular cut
                let cut_w = POCKET_RADIUS * 1.5;
                let cut_h = RAIL_WIDTH + 5.0;
                ctx.vectors.fill_rect(
                    Vec2::new(pos.x - cut_w / 2.0, if i == 4 { 0.0 } else { WORLD_H - cut_h }),
                    cut_w,
                    cut_h,
                    cut_color,
                );
            }
        }

        // Debug: show physics boundaries if enabled
        if DEBUG_DRAW {
            let debug_color = VectorColor::new(1.0, 0.0, 0.0, 0.5);
            let left = CUSHION;
            let right = WORLD_W - CUSHION;
            let top = CUSHION;
            let bottom = WORLD_H - CUSHION;
            ctx.vectors.stroke_polyline(&[
                Vec2::new(left, top),
                Vec2::new(right, top),
                Vec2::new(right, bottom),
                Vec2::new(left, bottom),
                Vec2::new(left, top),
            ], 2.0, debug_color);
        }
    }

    /// Draw the cue stick - positioned BEHIND cue ball, pointing toward target
    fn draw_cue_stick(&self, ctx: &mut EngineContext) {
        if !self.aiming {
            return;
        }

        let cue_pos = match self.cue_ball_pos(ctx) {
            Some(pos) => pos,
            None => return,
        };

        // aim_dir points from cue ball TOWARD where user is dragging (pull back direction)
        let aim_dir = (self.aim_current - cue_pos).normalize_or_zero();
        let pull_dist = cue_pos.distance(self.aim_current);

        // shot_dir is opposite - direction ball will travel
        let shot_dir = -aim_dir;

        // Cue stick is BEHIND the cue ball (in aim_dir direction, opposite to shot)
        let cue_length = 180.0;
        let retract = (pull_dist * 0.15).min(60.0);  // How far back the cue is pulled

        // Cue tip position - starts at ball edge, retracts as you pull
        let tip_pos = cue_pos + aim_dir * (BALL_RADIUS + 3.0 + retract);
        let butt_pos = tip_pos + aim_dir * cue_length;

        // Draw cue stick
        let cue_color = VectorColor::new(0.55, 0.35, 0.15, 1.0);  // Wood brown
        let tip_color = VectorColor::new(0.2, 0.4, 0.6, 1.0);     // Blue chalk tip

        // Main cue body (from tip toward butt)
        ctx.vectors.stroke_polyline(&[tip_pos, butt_pos], 5.0, cue_color);

        // Ferrule (white band near tip)
        let ferrule_start = tip_pos + aim_dir * 2.0;
        let ferrule_end = tip_pos + aim_dir * 10.0;
        ctx.vectors.stroke_polyline(&[ferrule_start, ferrule_end], 4.0, VectorColor::new(0.9, 0.9, 0.85, 1.0));

        // Tip (blue chalk)
        ctx.vectors.stroke_polyline(&[tip_pos, ferrule_start], 3.5, tip_color);

        // Power/aim indicator - line showing where ball will go
        if pull_dist > 15.0 {
            let indicator_color = VectorColor::new(1.0, 1.0, 1.0, 0.3);
            let indicator_end = cue_pos + shot_dir * (pull_dist * 0.8).min(300.0);
            ctx.vectors.stroke_polyline(
                &[cue_pos + shot_dir * (BALL_RADIUS + 3.0), indicator_end],
                1.5,
                indicator_color,
            );
        }
    }

    /// Spawn the cue ball
    fn spawn_cue_ball(&mut self, ctx: &mut EngineContext) {
        let id = ctx.next_id();
        let cue_def = &BALLS[0];

        let mesh = MeshComponent::pool_ball(BALL_RADIUS, cue_def.color);
        let entity = Entity::new(id)
            .with_tag("cue")
            .with_pos(Vec2::new(CUE_START_X, CUE_START_Y))
            .with_layer(RenderLayer::Objects)
            .with_mesh(mesh);

        let desc = BodyDesc::dynamic(ColliderDesc::Ball { radius: BALL_RADIUS })
            .with_position(Vec2::new(CUE_START_X, CUE_START_Y))
            .with_linear_damping(LINEAR_DAMPING)
            .with_angular_damping(ANGULAR_DAMPING)
            .with_ccd(true);

        let material = ColliderMaterial {
            restitution: RESTITUTION,
            friction: FRICTION,
            density: DENSITY,
        };

        ctx.spawn_with_body(entity, desc, material);
        self.cue_ball_id = Some(id);

        // Check if we're respawning (cue ball entry exists but is pocketed)
        if let Some(cue_entry) = self.balls.iter_mut().find(|b| b.ball_number == 0) {
            cue_entry.entity_id = id;
            cue_entry.pocketed = false;
            log::info!("Cue ball respawned");
        } else {
            // First spawn
            self.balls.push(BallEntity {
                entity_id: id,
                ball_number: 0,
                pocketed: false,
            });
        }
    }

    /// Spawn racked balls (1-15)
    fn spawn_rack(&mut self, ctx: &mut EngineContext) {
        let positions = rack_positions(Vec2::new(RACK_X, RACK_Y), BALL_RADIUS);

        for (i, &pos) in positions.iter().enumerate() {
            let ball_idx = i + 1;  // Balls 1-15
            let ball_def = &BALLS[ball_idx];
            let id = ctx.next_id();

            let mesh = match ball_def.ball_type {
                BallType::Cue => MeshComponent::pool_ball(BALL_RADIUS, ball_def.color),
                BallType::Solid => MeshComponent::pool_ball(BALL_RADIUS, ball_def.color),
                BallType::Striped => MeshComponent::striped_sphere(BALL_RADIUS, ball_def.color),
            };

            let entity = Entity::new(id)
                .with_tag(&format!("ball_{}", ball_def.number))
                .with_pos(pos)
                .with_layer(RenderLayer::Objects)
                .with_mesh(mesh);

            let desc = BodyDesc::dynamic(ColliderDesc::Ball { radius: BALL_RADIUS })
                .with_position(pos)
                .with_linear_damping(LINEAR_DAMPING)
                .with_angular_damping(ANGULAR_DAMPING)
                .with_ccd(true);

            let material = ColliderMaterial {
                restitution: RESTITUTION,
                friction: FRICTION,
                density: DENSITY,
            };

            ctx.spawn_with_body(entity, desc, material);

            self.balls.push(BallEntity {
                entity_id: id,
                ball_number: ball_def.number,
                pocketed: false,
            });
        }
    }

    /// Setup dynamic lighting - simple 3 overhead lights
    fn setup_lights(&self, ctx: &mut EngineContext) {
        use zap_engine::PointLight;

        ctx.lights.clear();
        // Bright ambient - table should be well lit
        ctx.lights.set_ambient(0.85, 0.83, 0.78);

        let warm = [1.0, 0.95, 0.85];  // Warm white
        let intensity = 0.4;
        let radius = 400.0;

        // Three evenly-spaced overhead lights
        let spacing = WORLD_W / 4.0;
        for i in 0..3 {
            let x = spacing + (i as f32) * spacing;
            ctx.lights.add(PointLight::new(
                Vec2::new(x, WORLD_H / 2.0),
                warm,
                intensity,
                radius,
            ));
        }
    }

    /// Reset the game
    fn reset(&mut self, ctx: &mut EngineContext) {
        // Despawn all balls
        for ball in &self.balls {
            ctx.despawn(ball.entity_id);
        }
        self.balls.clear();
        self.cue_ball_id = None;
        self.state = GameState::Aiming;
        self.aiming = false;
        ctx.effects.clear();

        // Respawn balls
        self.spawn_cue_ball(ctx);
        self.spawn_rack(ctx);
    }

    /// Get cue ball position
    fn cue_ball_pos(&self, ctx: &EngineContext) -> Option<Vec2> {
        self.cue_ball_id.and_then(|id| ctx.scene.get(id).map(|e| e.pos))
    }

    /// Check if all balls have stopped moving
    fn all_balls_stopped(&self, ctx: &EngineContext) -> bool {
        let threshold = 5.0;
        for ball in &self.balls {
            if ball.pocketed {
                continue;
            }
            let vel = ctx.velocity(ball.entity_id);
            if vel.length() > threshold {
                return false;
            }
        }
        true
    }

    /// Count remaining balls (excluding cue ball)
    fn balls_remaining(&self) -> u32 {
        self.balls.iter()
            .filter(|b| !b.pocketed && b.ball_number > 0)
            .count() as u32
    }

    /// Get pocket center positions (matching visual table)
    fn pocket_positions() -> [Vec2; 6] {
        [
            // Corner pockets (diagonal, at play area corners)
            Vec2::new(CORNER_POCKET_INSET, CORNER_POCKET_INSET),
            Vec2::new(WORLD_W - CORNER_POCKET_INSET, CORNER_POCKET_INSET),
            Vec2::new(CORNER_POCKET_INSET, WORLD_H - CORNER_POCKET_INSET),
            Vec2::new(WORLD_W - CORNER_POCKET_INSET, WORLD_H - CORNER_POCKET_INSET),
            // Side pockets (middle of top/bottom rails)
            Vec2::new(WORLD_W / 2.0, SIDE_POCKET_INSET),
            Vec2::new(WORLD_W / 2.0, WORLD_H - SIDE_POCKET_INSET),
        ]
    }

    /// Check if any balls are in pockets and despawn them
    fn check_pockets(&mut self, ctx: &mut EngineContext) {
        let pockets = Self::pocket_positions();
        let mut to_pocket = Vec::new();

        for (i, ball) in self.balls.iter().enumerate() {
            if ball.pocketed {
                continue;
            }
            if let Some(entity) = ctx.scene.get(ball.entity_id) {
                for &pocket_pos in &pockets {
                    let dist = entity.pos.distance(pocket_pos);
                    // Ball center within pocket radius = pocketed
                    if dist < POCKET_RADIUS - BALL_RADIUS * 0.3 {
                        to_pocket.push(i);
                        break;
                    }
                }
            }
        }

        // Pocket the balls (in reverse to preserve indices)
        for &i in to_pocket.iter().rev() {
            let ball = &mut self.balls[i];
            if ball.ball_number == 0 {
                // Cue ball - respawn it
                log::info!("Cue ball pocketed - respawning");
                ctx.despawn(ball.entity_id);
                ball.pocketed = true;
                self.cue_ball_id = None;
                // Will be respawned next frame when aiming
            } else {
                log::info!("Ball {} pocketed!", ball.ball_number);
                ctx.despawn(ball.entity_id);
                ball.pocketed = true;
            }
        }

        // Respawn cue ball if needed and we're aiming
        if self.cue_ball_id.is_none() && self.state == GameState::Aiming {
            self.spawn_cue_ball(ctx);
        }
    }

    /// Handle shot - drag away from target, release to shoot toward target
    fn shoot(&mut self, ctx: &mut EngineContext) {
        if let Some(cue_id) = self.cue_ball_id {
            if let Some(cue_pos) = self.cue_ball_pos(ctx) {
                // Direction from aim point back to cue ball = direction ball will travel
                let shot_dir = (cue_pos - self.aim_current).normalize_or_zero();
                let pull_dist = cue_pos.distance(self.aim_current);

                // Speed scales with pull distance
                let speed = (pull_dist * SHOT_SCALE).min(MAX_SHOT_SPEED);

                if speed > 20.0 && shot_dir.length() > 0.5 {
                    let velocity = shot_dir * speed;
                    log::info!("Setting velocity: {:?}", velocity);
                    ctx.set_velocity(cue_id, velocity);
                    self.state = GameState::BallsMoving;
                }
            }
        }
    }
}

impl Default for PoolGame {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for PoolGame {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: 1.0 / 60.0,
            world_width: WORLD_W,
            world_height: WORLD_H,
            max_instances: 64,
            max_sdf_instances: 32,  // 16 balls
            max_lights: 16,  // XXX pattern needs 11 lights
            gravity: Vec2::ZERO,  // Top-down view, no gravity
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        // Note: table sprite spawned in update() due to manifest loading timing
        Self::build_cushions(ctx);
        self.spawn_cue_ball(ctx);
        self.spawn_rack(ctx);
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        // Ensure felt sprite exists (deferred due to manifest loading)
        self.ensure_felt(ctx);

        // Clear vectors and draw table frame each frame
        ctx.vectors.clear();
        self.draw_table_frame(ctx);

        // Setup lights each frame
        self.setup_lights(ctx);

        // Handle input
        for event in input.iter() {
            match event {
                InputEvent::Custom { kind, .. } if *kind == events::RESET => {
                    self.reset(ctx);
                    return;
                }
                InputEvent::PointerDown { x, y } => {
                    if self.state == GameState::Aiming {
                        // Start aiming from anywhere on the table (easier to use)
                        self.aiming = true;
                        self.aim_start = Vec2::new(*x, *y);
                        self.aim_current = Vec2::new(*x, *y);
                    }
                }
                InputEvent::PointerMove { x, y } => {
                    if self.aiming {
                        self.aim_current = Vec2::new(*x, *y);
                    }
                }
                InputEvent::PointerUp { .. } => {
                    if self.aiming {
                        self.aiming = false;
                        self.shoot(ctx);
                    }
                }
                _ => {}
            }
        }

        // Check for pocketed balls
        self.check_pockets(ctx);

        // Update game state
        if self.state == GameState::BallsMoving && self.all_balls_stopped(ctx) {
            self.state = GameState::Aiming;
        }

        // Draw cue stick when aiming
        self.draw_cue_stick(ctx);

        // Emit game events
        ctx.emit_event(GameEvent {
            kind: game_events::BALLS_REMAINING,
            a: self.balls_remaining() as f32,
            b: 0.0,
            c: 0.0,
        });

        // Collision sparks (subtle)
        let spark_positions: Vec<Vec2> = ctx
            .collisions()
            .iter()
            .filter(|c| c.started)
            .filter_map(|c| {
                let a = ctx.scene.get(c.entity_a).map(|e| e.pos)?;
                let b = ctx.scene.get(c.entity_b).map(|e| e.pos)?;
                Some((a + b) / 2.0)
            })
            .collect();

        for mid in spark_positions {
            ctx.effects.spawn_particles(
                [mid.x, mid.y],
                2,      // count
                4.0,    // speed
                1.5,    // lifetime
                0.5,    // size
            );
        }
    }
}
