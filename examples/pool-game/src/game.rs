//! Pool game - 2D billiards with Rapier2D physics and SDF rendering.
//! Features pocket simulation where pocketed balls bounce and settle inside pockets.

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

// Table dimensions (the actual pool table)
const TABLE_W: f32 = 1000.0;
const TABLE_H: f32 = 500.0;

// Margin around table for cue stick drawing area
const TABLE_MARGIN: f32 = 80.0;

// World dimensions (table + margins for cue stick area)
const WORLD_W: f32 = TABLE_W + 2.0 * TABLE_MARGIN;  // 1300
const WORLD_H: f32 = TABLE_H + 2.0 * TABLE_MARGIN;  // 800

// Table offset within world (centered)
const TABLE_X: f32 = TABLE_MARGIN;
const TABLE_Y: f32 = TABLE_MARGIN;

// Table visual and physics
const RAIL_WIDTH: f32 = 35.0;  // Visual rail width
const CUSHION: f32 = 35.0;  // Physics boundary = rail width (ball EDGE touches rail)
const POCKET_GAP: f32 = 40.0;  // Gap in cushions at pockets

// Debug visualization
const DEBUG_DRAW: bool = false;

// Ball properties
const BALL_RADIUS: f32 = 12.0;

// Pocket properties - positioned at rail corners/sides
const POCKET_RADIUS: f32 = 22.0;  // Physics detection radius
const POCKET_VISUAL_RADIUS: f32 = 28.0;  // Visual hole size
const CORNER_POCKET_INSET: f32 = RAIL_WIDTH * 0.7;  // Corner pockets at rail corner
const SIDE_POCKET_INSET: f32 = RAIL_WIDTH * 0.5;    // Side pockets at rail edge

// Cue ball starting position (left side of table)
const CUE_START_X: f32 = TABLE_X + 250.0;
const CUE_START_Y: f32 = TABLE_Y + TABLE_H / 2.0;

// Rack apex position (right side of table)
const RACK_X: f32 = TABLE_X + 700.0;
const RACK_Y: f32 = TABLE_Y + TABLE_H / 2.0;

// Physics parameters
const LINEAR_DAMPING: f32 = 1.75;  // Felt friction (higher = stops faster)
const ANGULAR_DAMPING: f32 = 1.0;
const RESTITUTION: f32 = 0.95;     // Bouncy ball-to-ball
const FRICTION: f32 = 0.2;
const DENSITY: f32 = 0.01;  // Very low density so impulse = velocity

// Aiming - use velocity directly, not impulse
const MAX_SHOT_SPEED: f32 = 6400.0;
const SHOT_SCALE: f32 = 8.0;


// Pocket simulation constants
// Container radius = visual pocket radius - ball radius, so balls stay inside visually
const POCKET_CONTAINER_RADIUS: f32 = POCKET_VISUAL_RADIUS - BALL_RADIUS;  // ~16
const POCKET_DAMPING: f32 = 0.92;  // Velocity retention per frame (high damping to settle fast)
const POCKET_BALL_RESTITUTION: f32 = 0.6;  // Ball-ball bounce inside pocket
const POCKET_WALL_RESTITUTION: f32 = 0.4;  // Ball-wall bounce (soft pocket edge)
const POCKET_ENTRY_SPEED_SCALE: f32 = 0.15;  // Scale down incoming velocity
const POCKET_STOP_THRESHOLD: f32 = 2.0;  // Below this speed, ball stops

/// Custom event kinds from React UI
mod events {
    pub const RESET: u32 = 1;
}

/// Game event kinds to React
mod game_events {
    pub const BALLS_REMAINING: f32 = 1.0;
}

/// A ball inside a pocket with its own mini physics
#[derive(Debug, Clone)]
struct PocketedBall {
    ball_number: u8,
    entity_id: EntityId,
    pos: Vec2,      // Position relative to pocket center
    vel: Vec2,      // Velocity within pocket
}

/// A pocket container with its own physics simulation
#[derive(Debug, Clone)]
struct PocketContainer {
    center: Vec2,
    balls: Vec<PocketedBall>,
}

impl PocketContainer {
    fn new(center: Vec2) -> Self {
        Self {
            center,
            balls: Vec::new(),
        }
    }

    /// Add a ball entering the pocket with given world velocity
    fn add_ball(&mut self, ball_number: u8, entity_id: EntityId, world_vel: Vec2) {
        // Start at center, with scaled-down velocity
        let entry_vel = world_vel * POCKET_ENTRY_SPEED_SCALE;
        self.balls.push(PocketedBall {
            ball_number,
            entity_id,
            pos: Vec2::ZERO,  // Start at pocket center
            vel: entry_vel,
        });
    }

    /// Simulate one frame of pocket physics
    fn simulate(&mut self, dt: f32) {
        let ball_count = self.balls.len();
        if ball_count == 0 {
            return;
        }

        // Update positions
        for ball in &mut self.balls {
            ball.pos += ball.vel * dt;
        }

        // Ball-wall collisions (circular container)
        for ball in &mut self.balls {
            let dist = ball.pos.length();
            let max_dist = POCKET_CONTAINER_RADIUS;
            if dist > max_dist {
                // Push back inside and reflect velocity
                let normal = ball.pos.normalize_or_zero();
                ball.pos = normal * max_dist;
                // Reflect velocity off the wall
                let v_dot_n = ball.vel.dot(normal);
                if v_dot_n > 0.0 {
                    ball.vel -= normal * v_dot_n * (1.0 + POCKET_WALL_RESTITUTION);
                }
            }
        }

        // Ball-ball collisions (simple elastic)
        for i in 0..ball_count {
            for j in (i + 1)..ball_count {
                let pi = self.balls[i].pos;
                let pj = self.balls[j].pos;
                let delta = pj - pi;
                let dist = delta.length();
                let min_dist = BALL_RADIUS * 2.0;

                if dist < min_dist && dist > 0.001 {
                    // Separate balls
                    let normal = delta / dist;
                    let overlap = min_dist - dist;
                    let separation = normal * (overlap * 0.5);
                    self.balls[i].pos -= separation;
                    self.balls[j].pos += separation;

                    // Exchange velocity along collision normal
                    let vi = self.balls[i].vel;
                    let vj = self.balls[j].vel;
                    let rel_vel = vi - vj;
                    let v_along_normal = rel_vel.dot(normal);

                    if v_along_normal > 0.0 {
                        let impulse = normal * v_along_normal * POCKET_BALL_RESTITUTION;
                        self.balls[i].vel -= impulse;
                        self.balls[j].vel += impulse;
                    }
                }
            }
        }

        // Apply damping and stop slow balls
        for ball in &mut self.balls {
            ball.vel *= POCKET_DAMPING;
            if ball.vel.length() < POCKET_STOP_THRESHOLD {
                ball.vel = Vec2::ZERO;
            }
        }
    }

    /// Get world positions of all balls in this pocket
    fn world_positions(&self) -> impl Iterator<Item = (EntityId, Vec2)> + '_ {
        self.balls.iter().map(|b| (b.entity_id, self.center + b.pos))
    }

    /// Clear all balls (for reset)
    fn clear(&mut self) {
        self.balls.clear();
    }
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
    prev_pos: Vec2,  // Previous frame position for tunneling detection
}

pub struct PoolGame {
    state: GameState,
    aiming: bool,
    aim_start: Vec2,
    aim_current: Vec2,
    cue_ball_id: Option<EntityId>,
    balls: Vec<BallEntity>,
    table_id: Option<EntityId>,
    /// 6 pocket containers with mini physics simulation
    pockets: [PocketContainer; 6],
}

impl PoolGame {
    pub fn new() -> Self {
        // Initialize pocket containers at their positions
        let pocket_positions = Self::pocket_positions();
        Self {
            state: GameState::Aiming,
            aiming: false,
            aim_start: Vec2::ZERO,
            aim_current: Vec2::ZERO,
            cue_ball_id: None,
            balls: Vec::with_capacity(16),
            table_id: None,
            pockets: [
                PocketContainer::new(pocket_positions[0]),
                PocketContainer::new(pocket_positions[1]),
                PocketContainer::new(pocket_positions[2]),
                PocketContainer::new(pocket_positions[3]),
                PocketContainer::new(pocket_positions[4]),
                PocketContainer::new(pocket_positions[5]),
            ],
        }
    }

    /// Ensure felt sprites exist (2 tiles side-by-side, play area only)
    fn ensure_felt(&mut self, ctx: &mut EngineContext) {
        if self.table_id.is_some() {
            return;
        }

        let sprite = match ctx.sprite("felt") {
            Some(s) => s,
            None => return,  // Manifest not loaded yet, try again next frame
        };

        // Play area bounds (inside the rails) - this is where felt should show
        let play_left = TABLE_X + RAIL_WIDTH;
        let play_top = TABLE_Y + RAIL_WIDTH;
        let play_w = TABLE_W - 2.0 * RAIL_WIDTH;  // 930
        let play_h = TABLE_H - 2.0 * RAIL_WIDTH;  // 430
        let center_y = play_top + play_h / 2.0;

        // Use 2 tiles side by side (each 465x465, covering half width)
        // Vertical overflow (~17px) is covered by the rails
        let tile_size = play_w / 2.0;  // 465

        let mut first_id = None;
        let tile_positions = [
            Vec2::new(play_left + tile_size * 0.5, center_y),
            Vec2::new(play_left + tile_size * 1.5, center_y),
        ];
        for (i, pos) in tile_positions.iter().enumerate() {
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
        log::info!("Felt sprites spawned (2 tiles, size={})", tile_size);
    }

    /// Build table cushion walls with gaps for pockets
    fn build_cushions(ctx: &mut EngineContext) {
        let wall_material = ColliderMaterial {
            restitution: 0.95,  // Cushion bounce - 95% energy retention
            friction: 0.2,
            density: 1.0,
        };

        // Play area boundaries (relative to table origin)
        let left = TABLE_X + CUSHION;
        let right = TABLE_X + TABLE_W - CUSHION;
        let top = TABLE_Y + CUSHION;
        let bottom = TABLE_Y + TABLE_H - CUSHION;
        let mid_x = TABLE_X + TABLE_W / 2.0;
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
            .with_position(Vec2::new(left + corner_gap + seg_len / 2.0, TABLE_Y + CUSHION / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }
        // Right segment
        if seg_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: seg_len / 2.0,
                half_height: CUSHION / 2.0,
            })
            .with_position(Vec2::new(right - corner_gap - seg_len / 2.0, TABLE_Y + CUSHION / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }

        // Bottom cushion - same pattern
        if seg_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: seg_len / 2.0,
                half_height: CUSHION / 2.0,
            })
            .with_position(Vec2::new(left + corner_gap + seg_len / 2.0, TABLE_Y + TABLE_H - CUSHION / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }
        if seg_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: seg_len / 2.0,
                half_height: CUSHION / 2.0,
            })
            .with_position(Vec2::new(right - corner_gap - seg_len / 2.0, TABLE_Y + TABLE_H - CUSHION / 2.0));
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
            .with_position(Vec2::new(TABLE_X + CUSHION / 2.0, TABLE_Y + TABLE_H / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }

        // Right cushion - same pattern
        if side_len > 0.0 {
            let id = ctx.next_id();
            let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
                half_width: CUSHION / 2.0,
                half_height: side_len / 2.0,
            })
            .with_position(Vec2::new(TABLE_X + TABLE_W - CUSHION / 2.0, TABLE_Y + TABLE_H / 2.0));
            ctx.spawn_with_body(Entity::new(id).with_tag("cushion"), desc, wall_material);
        }
    }

    /// Draw table frame (rails, pockets with cuts) with vectors
    fn draw_table_frame(&self, ctx: &mut EngineContext) {
        // Colors
        let rail_color = VectorColor::new(0.12, 0.06, 0.02, 1.0);  // Dark brown wood
        let pocket_color = VectorColor::new(0.02, 0.02, 0.02, 1.0);  // Near-black pocket holes

        // Draw pocket holes FIRST (underneath rails)
        let pockets = Self::pocket_positions();
        for pos in &pockets {
            ctx.vectors.fill_circle(*pos, POCKET_VISUAL_RADIUS, pocket_color);
        }

        // Draw rails (relative to table position)
        // Top rail
        ctx.vectors.fill_rect(Vec2::new(TABLE_X, TABLE_Y), TABLE_W, RAIL_WIDTH, rail_color);
        // Bottom rail
        ctx.vectors.fill_rect(Vec2::new(TABLE_X, TABLE_Y + TABLE_H - RAIL_WIDTH), TABLE_W, RAIL_WIDTH, rail_color);
        // Left rail
        ctx.vectors.fill_rect(Vec2::new(TABLE_X, TABLE_Y + RAIL_WIDTH), RAIL_WIDTH, TABLE_H - 2.0 * RAIL_WIDTH, rail_color);
        // Right rail
        ctx.vectors.fill_rect(Vec2::new(TABLE_X + TABLE_W - RAIL_WIDTH, TABLE_Y + RAIL_WIDTH), RAIL_WIDTH, TABLE_H - 2.0 * RAIL_WIDTH, rail_color);

        // Draw pocket holes AGAIN on top (to cut into rails)
        for pos in &pockets {
            ctx.vectors.fill_circle(*pos, POCKET_VISUAL_RADIUS, pocket_color);
        }

        // Debug: show physics boundaries if enabled
        if DEBUG_DRAW {
            let debug_color = VectorColor::new(1.0, 0.0, 0.0, 0.5);
            let left = TABLE_X + CUSHION;
            let right = TABLE_X + TABLE_W - CUSHION;
            let top = TABLE_Y + CUSHION;
            let bottom = TABLE_Y + TABLE_H - CUSHION;
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
                prev_pos: Vec2::new(CUE_START_X, CUE_START_Y),
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
                prev_pos: pos,
            });
        }
    }

    /// Setup dynamic lighting - main table lights + dim pocket lights
    /// Main lights affect all layers except Terrain (0x3D), pocket lights affect only Terrain (0x02)
    fn setup_lights(&self, ctx: &mut EngineContext) {
        use zap_engine::PointLight;

        ctx.lights.clear();
        // Moderate ambient for main table
        ctx.lights.set_ambient(0.55, 0.53, 0.50);

        let warm = [1.0, 0.95, 0.85];  // Warm white
        let intensity = 0.8;
        let radius = 450.0;

        // Layer masks: Terrain = 0x02, everything else = 0x3D (0x3F minus 0x02)
        let main_layer_mask = 0x3D;  // Affects all except Terrain (pocketed balls)
        let pocket_layer_mask = 0x02;  // Affects only Terrain (pocketed balls)

        // Three evenly-spaced overhead lights (over the table) - main lights
        let spacing = TABLE_W / 4.0;
        for i in 0..3 {
            let x = TABLE_X + spacing + (i as f32) * spacing;
            ctx.lights.add(PointLight::new(
                Vec2::new(x, TABLE_Y + TABLE_H / 2.0),
                warm,
                intensity,
                radius,
            ).with_layer_mask(main_layer_mask));
        }

        // Dim pocket lights - one at each pocket, very low intensity
        let pockets = Self::pocket_positions();
        let pocket_intensity = 0.15;  // Much dimmer - balls are in shadow
        let pocket_radius = 80.0;
        for pocket_pos in &pockets {
            ctx.lights.add(PointLight::new(
                *pocket_pos,
                warm,
                pocket_intensity,
                pocket_radius,
            ).with_layer_mask(pocket_layer_mask));
        }
    }

    /// Reset the game
    fn reset(&mut self, ctx: &mut EngineContext) {
        // Despawn all balls (including those on table)
        for ball in &self.balls {
            ctx.despawn(ball.entity_id);
        }
        self.balls.clear();
        self.cue_ball_id = None;
        self.state = GameState::Aiming;
        self.aiming = false;
        ctx.effects.clear();

        // Clear pocket containers and despawn their visual entities
        for pocket in &mut self.pockets {
            for pocketed_ball in &pocket.balls {
                ctx.despawn(pocketed_ball.entity_id);
            }
            pocket.clear();
        }

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
            Vec2::new(TABLE_X + CORNER_POCKET_INSET, TABLE_Y + CORNER_POCKET_INSET),
            Vec2::new(TABLE_X + TABLE_W - CORNER_POCKET_INSET, TABLE_Y + CORNER_POCKET_INSET),
            Vec2::new(TABLE_X + CORNER_POCKET_INSET, TABLE_Y + TABLE_H - CORNER_POCKET_INSET),
            Vec2::new(TABLE_X + TABLE_W - CORNER_POCKET_INSET, TABLE_Y + TABLE_H - CORNER_POCKET_INSET),
            // Side pockets (middle of top/bottom rails)
            Vec2::new(TABLE_X + TABLE_W / 2.0, TABLE_Y + SIDE_POCKET_INSET),
            Vec2::new(TABLE_X + TABLE_W / 2.0, TABLE_Y + TABLE_H - SIDE_POCKET_INSET),
        ]
    }

    /// Check if a line segment passes within radius of a point (for tunneling detection)
    fn segment_point_distance(p1: Vec2, p2: Vec2, point: Vec2) -> f32 {
        let line = p2 - p1;
        let len_sq = line.length_squared();
        if len_sq < 0.0001 {
            return p1.distance(point);
        }
        // Project point onto line, clamped to segment
        let t = ((point - p1).dot(line) / len_sq).clamp(0.0, 1.0);
        let projection = p1 + line * t;
        projection.distance(point)
    }

    /// Check if a ball has escaped the play area and handle it.
    /// Simple position check - if ball center is outside the cushion boundaries, it escaped.
    fn check_escaped_balls(&mut self, ctx: &mut EngineContext) {
        // Play area bounds - inside the cushions
        let left = TABLE_X + CUSHION;           // 115
        let right = TABLE_X + TABLE_W - CUSHION; // 1045
        let top = TABLE_Y + CUSHION;            // 115
        let bottom = TABLE_Y + TABLE_H - CUSHION; // 545

        let pockets = Self::pocket_positions();

        // Collect escaped balls
        let mut escaped: Vec<(usize, Vec2)> = Vec::new();
        for (i, ball) in self.balls.iter().enumerate() {
            if ball.pocketed {
                continue;
            }
            if let Some(entity) = ctx.scene.get(ball.entity_id) {
                let pos = entity.pos;
                if pos.x < left || pos.x > right || pos.y < top || pos.y > bottom {
                    escaped.push((i, pos));
                }
            }
        }

        // Handle escaped balls
        for (i, escaped_pos) in escaped.iter().rev() {
            let ball = &mut self.balls[*i];
            let ball_number = ball.ball_number;
            let old_id = ball.entity_id;

            if ball_number == 0 {
                // Cue ball escaped - despawn and respawn
                log::warn!("Cue ball escaped at {:?} - respawning", escaped_pos);
                ctx.despawn(old_id);
                ball.pocketed = true;
                self.cue_ball_id = None;
            } else {
                // Numbered ball escaped - find nearest pocket and pocket it there
                let nearest_pocket = pockets.iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.distance(*escaped_pos).partial_cmp(&b.distance(*escaped_pos)).unwrap()
                    })
                    .map(|(idx, _)| idx)
                    .unwrap_or(0);

                log::warn!("Ball {} escaped at {:?} - pocketing to pocket {}",
                    ball_number, escaped_pos, nearest_pocket);
                ctx.despawn(old_id);

                // Spawn visual-only ball at the pocket on Terrain layer
                let ball_def = &BALLS[ball_number as usize];
                let mesh = match ball_def.ball_type {
                    BallType::Cue => MeshComponent::pool_ball(BALL_RADIUS, ball_def.color),
                    BallType::Solid => MeshComponent::pool_ball(BALL_RADIUS, ball_def.color),
                    BallType::Striped => MeshComponent::striped_sphere(BALL_RADIUS, ball_def.color),
                };

                let new_id = ctx.next_id();
                let pocket_pos = pockets[nearest_pocket];
                let entity = Entity::new(new_id)
                    .with_tag(&format!("pocketed_{}", ball_number))
                    .with_pos(pocket_pos)
                    .with_layer(RenderLayer::Terrain)
                    .with_mesh(mesh);
                ctx.scene.spawn(entity);

                // Add to pocket simulation with zero velocity (it just dropped in)
                self.pockets[nearest_pocket].add_ball(ball_number, new_id, Vec2::ZERO);

                ball.entity_id = new_id;
                ball.pocketed = true;
            }
        }

        // Respawn cue ball if needed and we're aiming
        if self.cue_ball_id.is_none() && self.state == GameState::Aiming {
            self.spawn_cue_ball(ctx);
        }
    }

    /// Check if any balls are in pockets - uses trajectory detection for fast balls
    fn check_pockets(&mut self, ctx: &mut EngineContext) {
        let pockets = Self::pocket_positions();
        // (ball_index, pocket_index, ball_velocity)
        let mut to_pocket: Vec<(usize, usize, Vec2)> = Vec::new();

        for (i, ball) in self.balls.iter().enumerate() {
            if ball.pocketed {
                continue;
            }
            if let Some(entity) = ctx.scene.get(ball.entity_id) {
                let current_pos = entity.pos;
                let prev_pos = ball.prev_pos;
                let vel = ctx.velocity(ball.entity_id);

                for (pocket_idx, &pocket_pos) in pockets.iter().enumerate() {
                    // Check if ball trajectory crossed the pocket (handles fast balls)
                    let dist = Self::segment_point_distance(prev_pos, current_pos, pocket_pos);
                    if dist < POCKET_RADIUS + BALL_RADIUS * 0.5 {
                        to_pocket.push((i, pocket_idx, vel));
                        break;
                    }
                }
            }
        }

        // Update prev_pos for all balls
        for ball in &mut self.balls {
            if !ball.pocketed {
                if let Some(entity) = ctx.scene.get(ball.entity_id) {
                    ball.prev_pos = entity.pos;
                }
            }
        }

        // Pocket the balls (in reverse to preserve indices)
        for (i, pocket_idx, vel) in to_pocket.iter().rev() {
            let ball = &mut self.balls[*i];
            let ball_number = ball.ball_number;
            let old_id = ball.entity_id;

            if ball_number == 0 {
                // Cue ball - despawn and respawn later
                log::info!("Cue ball pocketed - respawning");
                ctx.despawn(old_id);
                ball.pocketed = true;
                self.cue_ball_id = None;
            } else {
                // Numbered ball - despawn physics ball, spawn visual on Terrain layer
                // and add to pocket simulation
                log::info!("Ball {} pocketed into pocket {}!", ball_number, pocket_idx);
                ctx.despawn(old_id);

                // Spawn a new visual-only ball at the pocket center on Terrain layer
                let ball_def = &BALLS[ball_number as usize];
                let mesh = match ball_def.ball_type {
                    BallType::Cue => MeshComponent::pool_ball(BALL_RADIUS, ball_def.color),
                    BallType::Solid => MeshComponent::pool_ball(BALL_RADIUS, ball_def.color),
                    BallType::Striped => MeshComponent::striped_sphere(BALL_RADIUS, ball_def.color),
                };

                let new_id = ctx.next_id();
                let pocket_pos = pockets[*pocket_idx];
                let entity = Entity::new(new_id)
                    .with_tag(&format!("pocketed_{}", ball_number))
                    .with_pos(pocket_pos)
                    .with_layer(RenderLayer::Terrain)  // Terrain layer gets dimmer lights
                    .with_mesh(mesh);
                ctx.scene.spawn(entity);

                // Add to pocket simulation with incoming velocity
                self.pockets[*pocket_idx].add_ball(ball_number, new_id, *vel);

                ball.entity_id = new_id;
                ball.pocketed = true;
            }
        }

        // Respawn cue ball if needed and we're aiming
        if self.cue_ball_id.is_none() && self.state == GameState::Aiming {
            self.spawn_cue_ball(ctx);
        }
    }

    /// Simulate physics for balls inside pockets
    fn simulate_pockets(&mut self, ctx: &mut EngineContext, dt: f32) {
        for pocket in &mut self.pockets {
            pocket.simulate(dt);

            // Update visual entity positions
            for (entity_id, world_pos) in pocket.world_positions() {
                if let Some(entity) = ctx.scene.get_mut(entity_id) {
                    entity.pos = world_pos;
                }
            }
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
            // 240Hz physics for precise ball collisions (4 substeps × 60Hz = 240Hz)
            physics_substeps: 4,
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

        // Check for escaped balls (tunneled through cushions on slow devices)
        self.check_escaped_balls(ctx);

        // Simulate pocketed balls in their pocket containers
        let dt = self.config().fixed_dt;
        self.simulate_pockets(ctx, dt);

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
