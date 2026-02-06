use zap_engine::*;
use zap_engine::api::game::GameConfig;
use zap_engine::components::sprite::{SpriteComponent, AtlasId, BlendMode};
use zap_engine::input::queue::{InputEvent, InputQueue};
use zap_engine::systems::effects::SegmentColor;
use glam::Vec2;

const WORLD_W: f32 = 1200.0;
const WORLD_H: f32 = 600.0;
const GROUND_Y: f32 = 550.0;
const GROUND_THICKNESS: f32 = 50.0;
const WALL_THICKNESS: f32 = 20.0;

const BLOCK_HALF_W: f32 = 20.0;
const BLOCK_HALF_H: f32 = 20.0;
const BLOCK_SIZE: f32 = BLOCK_HALF_W * 2.0;

const SLING_X: f32 = 400.0;
const SLING_Y: f32 = 450.0;
const BALL_RADIUS: f32 = 15.0;
const LAUNCH_SCALE: f32 = 4.0;

const TOWER_BASE_X: f32 = 800.0;
const TOWER_ROWS: usize = 5;
const TOWER_COLS: usize = 3;

const SETTLED_FRAMES: u32 = 60;
const SETTLED_VEL_THRESHOLD: f32 = 15.0;
/// Maximum flight time before auto-respawn (frames). Prevents stuck projectiles.
const MAX_FLIGHT_FRAMES: u32 = 600;

/// Custom event kinds from React UI
const CUSTOM_RESET: u32 = 1;

/// Game event kinds to React
const EVENT_SCORE: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum GameState {
    Aiming,
    Flying,
    Settled,
}

pub struct PhysicsPlayground {
    state: GameState,
    dragging: bool,
    drag_start: Vec2,
    drag_current: Vec2,
    projectile_id: Option<EntityId>,
    block_ids: Vec<EntityId>,
    settled_counter: u32,
    flight_timer: u32,
    score: u32,
}

impl PhysicsPlayground {
    pub fn new() -> Self {
        Self {
            state: GameState::Aiming,
            dragging: false,
            drag_start: Vec2::ZERO,
            drag_current: Vec2::ZERO,
            projectile_id: None,
            block_ids: Vec::new(),
            settled_counter: 0,
            flight_timer: 0,
            score: 0,
        }
    }

    fn build_ground(ctx: &mut EngineContext) {
        let id = ctx.next_id();
        let entity = Entity::new(id)
            .with_tag("ground")
            .with_pos(Vec2::new(WORLD_W / 2.0, GROUND_Y + GROUND_THICKNESS / 2.0));
        let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
            half_width: WORLD_W / 2.0,
            half_height: GROUND_THICKNESS / 2.0,
        })
        .with_position(Vec2::new(WORLD_W / 2.0, GROUND_Y + GROUND_THICKNESS / 2.0));
        ctx.spawn_with_body(entity, desc, ColliderMaterial::default());
    }

    fn build_walls(ctx: &mut EngineContext) {
        // Left wall
        let id = ctx.next_id();
        let entity = Entity::new(id).with_tag("wall");
        let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
            half_width: WALL_THICKNESS / 2.0,
            half_height: WORLD_H / 2.0,
        })
        .with_position(Vec2::new(-WALL_THICKNESS / 2.0, WORLD_H / 2.0));
        ctx.spawn_with_body(entity, desc, ColliderMaterial::default());

        // Right wall
        let id = ctx.next_id();
        let entity = Entity::new(id).with_tag("wall");
        let desc = BodyDesc::fixed(ColliderDesc::Cuboid {
            half_width: WALL_THICKNESS / 2.0,
            half_height: WORLD_H / 2.0,
        })
        .with_position(Vec2::new(WORLD_W + WALL_THICKNESS / 2.0, WORLD_H / 2.0));
        ctx.spawn_with_body(entity, desc, ColliderMaterial::default());
    }

    fn build_tower(&mut self, ctx: &mut EngineContext) {
        self.block_ids.clear();
        for row in 0..TOWER_ROWS {
            for col in 0..TOWER_COLS {
                let x = TOWER_BASE_X + col as f32 * BLOCK_SIZE;
                let y = GROUND_Y - BLOCK_HALF_H - (row as f32 * BLOCK_SIZE);
                let id = ctx.next_id();

                // Vary sprite column based on position for visual variety
                let sprite_col = ((row + col) % 4) as f32;
                let entity = Entity::new(id)
                    .with_tag("block")
                    .with_pos(Vec2::new(x, y))
                    .with_scale(Vec2::splat(BLOCK_SIZE))
                    .with_sprite(SpriteComponent {
                        atlas: AtlasId(0),
                        col: sprite_col,
                        row: 0.0,
                        cell_span: 1.0,
                        alpha: 1.0,
                        blend: BlendMode::Alpha,
                    });

                let desc = BodyDesc::dynamic(ColliderDesc::Cuboid {
                    half_width: BLOCK_HALF_W,
                    half_height: BLOCK_HALF_H,
                })
                .with_position(Vec2::new(x, y));

                let material = ColliderMaterial {
                    restitution: 0.2,
                    friction: 0.6,
                    density: 0.5,
                };

                ctx.spawn_with_body(entity, desc, material);
                self.block_ids.push(id);
            }
        }
    }

    fn reset_level(&mut self, ctx: &mut EngineContext) {
        // Clear accumulated effects (arcs + particles)
        ctx.effects.clear();

        // Despawn all entities except walls, ground, and sling indicator
        let to_despawn: Vec<EntityId> = ctx.scene.iter()
            .filter(|e| {
                let tag = e.tag.as_str();
                tag != "wall" && tag != "ground" && tag != "sling"
            })
            .map(|e| e.id)
            .collect();

        for id in to_despawn {
            ctx.despawn(id);
        }

        self.projectile_id = None;
        self.block_ids.clear();
        self.state = GameState::Aiming;
        self.dragging = false;
        self.settled_counter = 0;
        self.score = 0;

        self.build_tower(ctx);
    }

    fn launch_projectile(&mut self, ctx: &mut EngineContext) {
        let pull = Vec2::new(SLING_X, SLING_Y) - self.drag_current;
        let velocity = pull * LAUNCH_SCALE;

        let id = ctx.next_id();
        let entity = Entity::new(id)
            .with_tag("projectile")
            .with_pos(Vec2::new(SLING_X, SLING_Y))
            .with_scale(Vec2::splat(BALL_RADIUS * 2.0))
            .with_sprite(SpriteComponent {
                atlas: AtlasId(0),
                col: 3.0,
                row: 0.0,
                cell_span: 1.0,
                alpha: 1.0,
                blend: BlendMode::Alpha,
            });

        let desc = BodyDesc::dynamic(ColliderDesc::Ball { radius: BALL_RADIUS })
            .with_position(Vec2::new(SLING_X, SLING_Y))
            .with_velocity(velocity)
            .with_ccd(true);

        let material = ColliderMaterial {
            restitution: 0.3,
            friction: 0.5,
            density: 2.0,
        };

        ctx.spawn_with_body(entity, desc, material);
        self.projectile_id = Some(id);
        self.state = GameState::Flying;
        self.settled_counter = 0;
        self.flight_timer = 0;
    }

    fn count_knocked_blocks(&self, ctx: &EngineContext) -> u32 {
        let threshold = GROUND_Y + BLOCK_SIZE;
        let mut count = 0u32;
        for &id in &self.block_ids {
            if let Some(entity) = ctx.scene.get(id) {
                // Knocked if fallen below ground or rotated significantly
                if entity.pos.y > threshold || entity.rotation.abs() > 0.3 {
                    count += 1;
                }
            } else {
                // Entity was despawned somehow
                count += 1;
            }
        }
        count
    }

    fn draw_sling_band(&mut self, ctx: &mut EngineContext) {
        if self.dragging {
            let origin = [SLING_X, SLING_Y];
            let target = [self.drag_current.x, self.drag_current.y];
            ctx.effects.add_arc(origin, target, 2.0, SegmentColor::White, 4);
        }
    }
}

impl Game for PhysicsPlayground {
    fn config(&self) -> GameConfig {
        GameConfig {
            fixed_dt: 1.0 / 60.0,
            world_width: WORLD_W,
            world_height: WORLD_H,
            max_instances: 256,
            gravity: Vec2::new(0.0, 600.0),
            ..GameConfig::default()
        }
    }

    fn init(&mut self, ctx: &mut EngineContext) {
        Self::build_ground(ctx);
        Self::build_walls(ctx);
        self.build_tower(ctx);

        // Sling origin indicator — semi-transparent marker showing where to drag from
        let sling_id = ctx.next_id();
        ctx.scene.spawn(
            Entity::new(sling_id)
                .with_tag("sling")
                .with_pos(Vec2::new(SLING_X, SLING_Y))
                .with_scale(Vec2::splat(BALL_RADIUS * 3.0))
                .with_sprite(SpriteComponent {
                    atlas: AtlasId(0),
                    col: 3.0,
                    row: 0.0,
                    cell_span: 1.0,
                    alpha: 0.4,
                    blend: BlendMode::Alpha,
                }),
        );
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        // Clear previous frame's arcs (arcs persist in EffectsState)
        ctx.effects.arcs.clear();

        // Handle input events
        for event in input.iter() {
            match event {
                InputEvent::Custom { kind, .. } if *kind == CUSTOM_RESET => {
                    self.reset_level(ctx);
                    return;
                }
                InputEvent::PointerDown { x, y } => {
                    if self.state == GameState::Aiming {
                        let pos = Vec2::new(*x, *y);
                        let dist = pos.distance(Vec2::new(SLING_X, SLING_Y));
                        if dist < 100.0 {
                            self.dragging = true;
                            self.drag_start = pos;
                            self.drag_current = pos;
                        }
                    }
                }
                InputEvent::PointerMove { x, y } => {
                    if self.dragging {
                        self.drag_current = Vec2::new(*x, *y);
                    }
                }
                InputEvent::PointerUp { x, y } => {
                    if self.dragging {
                        self.drag_current = Vec2::new(*x, *y);
                        self.dragging = false;
                        self.launch_projectile(ctx);
                    }
                }
                _ => {}
            }
        }

        // Draw sling band during aiming
        self.draw_sling_band(ctx);

        // Flying state: check if projectile settled or flight timed out
        if self.state == GameState::Flying {
            self.flight_timer += 1;
            if let Some(proj_id) = self.projectile_id {
                let vel = ctx.velocity(proj_id);
                if vel.length() < SETTLED_VEL_THRESHOLD {
                    self.settled_counter += 1;
                } else {
                    self.settled_counter = 0;
                }
                if self.settled_counter >= SETTLED_FRAMES || self.flight_timer >= MAX_FLIGHT_FRAMES {
                    self.state = GameState::Settled;
                }
            }
        }

        // Settled state: despawn old projectile and allow next shot
        if self.state == GameState::Settled {
            if let Some(pid) = self.projectile_id.take() {
                ctx.despawn(pid);
            }
            self.state = GameState::Aiming;
            self.settled_counter = 0;
        }

        // Count score
        self.score = self.count_knocked_blocks(ctx);
        ctx.emit_event(GameEvent {
            kind: EVENT_SCORE,
            a: self.score as f32,
            b: 0.0,
            c: 0.0,
        });

        // Collision sparks
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
                4,
                6.0,
                2.5,
                0.8,
            );
        }

        ctx.effects.attractor = [SLING_X, GROUND_Y];
    }
}
