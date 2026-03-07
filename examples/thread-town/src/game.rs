//! Thread Town game implementation.

use glam::Vec2;
use zap_engine::*;
use zap_engine::input::queue::InputEvent;

// ── World layout ─────────────────────────────────────────────────────
const WORLD_W: f32 = 800.0;
const WORLD_H: f32 = 600.0;
const FIXED_DT: f32 = 1.0 / 60.0;

// Entity positions
const COUNTER_POS: Vec2 = Vec2::new(400.0, 180.0);
const ROBOT_A_HOME: Vec2 = Vec2::new(150.0, 400.0);
const ROBOT_B_HOME: Vec2 = Vec2::new(650.0, 400.0);
const COUNTER_APPROACH_A: Vec2 = Vec2::new(300.0, 250.0);
const COUNTER_APPROACH_B: Vec2 = Vec2::new(500.0, 250.0);
const LOCK_POS: Vec2 = Vec2::new(400.0, 320.0);
const WAITING_POS_B: Vec2 = Vec2::new(550.0, 350.0);

// Timing
const STEP_DURATION: f32 = 0.8;
const WALK_DURATION: f32 = 0.5;

// Custom events from React
const CUSTOM_PLAY: u32 = 1;
const CUSTOM_PAUSE: u32 = 2;
const CUSTOM_STEP: u32 = 3;
const CUSTOM_RESET: u32 = 4;
const CUSTOM_SCENARIO: u32 = 5;

// Game events to React
const EVENT_STATE_CHANGE: f32 = 1.0;
const EVENT_COUNTER_VALUE: f32 = 2.0;
const EVENT_RACE_DETECTED: f32 = 3.0;
const EVENT_PHASE_NAME: f32 = 4.0;
const EVENT_SUCCESS: f32 = 5.0;
const EVENT_SCENARIO: f32 = 6.0;

// ── Robot state ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum RobotState {
    Idle,
    WalkingToCounter,
    AtCounter,
    Reading,
    Incrementing,
    Writing,
    WalkingHome,
    WaitingForLock,
    HoldingLock,
}

#[derive(Debug, Clone, Copy)]
struct RobotData {
    entity_id: EntityId,
    state: RobotState,
    register: i32,
    thought_id: Option<EntityId>,
    home_pos: Vec2,
    counter_pos: Vec2,
}

impl RobotData {
    fn new(entity_id: EntityId, home: Vec2, counter: Vec2) -> Self {
        Self {
            entity_id,
            state: RobotState::Idle,
            register: 0,
            thought_id: None,
            home_pos: home,
            counter_pos: counter,
        }
    }
}

// ── Scenario phases ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Scenario {
    DataRace,
    MutexFix,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DataRacePhase {
    Initial,
    BothWalkToCounter,
    BothAtCounter,
    AReads,
    BReads,
    AIncrements,
    BIncrements,
    AWrites,
    BWrites,
    RaceDetected,
    BothWalkHome,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum MutexPhase {
    Initial,
    AWalksToCounter,
    AAcquiresLock,
    AReads,
    AIncrements,
    AWrites,
    AReleasesLock,
    BWalksToCounter,
    BAcquiresLock,
    BReads,
    BIncrements,
    BWrites,
    BReleasesLock,
    BothWalkHome,
    Complete,
}

// ── Main game struct ─────────────────────────────────────────────────

pub struct ThreadTownGame {
    // Subsystems
    tweens: TweenState,

    // Entities
    robot_a: RobotData,
    robot_b: RobotData,
    counter_value: i32,
    counter_entity: EntityId,
    digit_entities: Vec<EntityId>,
    lock_entity: Option<EntityId>,
    lock_closed: bool,
    waiting_indicator: Option<EntityId>,

    // UI state
    playing: bool,
    step_timer: f32,

    // Scenario state
    scenario: Scenario,
    data_race_phase: DataRacePhase,
    mutex_phase: MutexPhase,

    // For sending phase names to React
    last_phase_sent: i32,

    // Visible dimensions
    visible_w: f32,
    visible_h: f32,

    // Track entity IDs
    next_id: u32,
    initialized: bool,
}

impl ThreadTownGame {
    pub fn new() -> Self {
        Self {
            tweens: TweenState::new(),
            robot_a: RobotData::new(EntityId(10), ROBOT_A_HOME, COUNTER_APPROACH_A),
            robot_b: RobotData::new(EntityId(11), ROBOT_B_HOME, COUNTER_APPROACH_B),
            counter_value: 5,
            counter_entity: EntityId(20),
            digit_entities: Vec::new(),
            lock_entity: None,
            lock_closed: false,
            waiting_indicator: None,
            playing: false,
            step_timer: 0.0,
            scenario: Scenario::DataRace,
            data_race_phase: DataRacePhase::Initial,
            mutex_phase: MutexPhase::Initial,
            last_phase_sent: -1,
            visible_w: WORLD_W,
            visible_h: WORLD_H,
            next_id: 100,
            initialized: false,
        }
    }

    fn next_entity_id(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    fn setup_scene(&mut self, ctx: &mut EngineContext) {
        // Background
        if let Some(bg_sprite) = ctx.sprite("background") {
            let bg_size = self.visible_w.max(self.visible_h) * 1.5;
            ctx.scene.spawn(
                Entity::new(EntityId(1))
                    .with_pos(Vec2::new(self.visible_w / 2.0, self.visible_h / 2.0))
                    .with_scale(Vec2::splat(bg_size))
                    .with_sprite(bg_sprite)
                    .with_layer(RenderLayer::Background)
            );
        }

        // Robot A (blue)
        if let Some(sprite) = ctx.sprite("robot_blue") {
            ctx.scene.spawn(
                Entity::new(self.robot_a.entity_id)
                    .with_pos(ROBOT_A_HOME)
                    .with_scale(Vec2::splat(64.0))
                    .with_sprite(sprite)
                    .with_layer(RenderLayer::Objects)
            );
        }

        // Robot B (orange)
        if let Some(sprite) = ctx.sprite("robot_orange") {
            ctx.scene.spawn(
                Entity::new(self.robot_b.entity_id)
                    .with_pos(ROBOT_B_HOME)
                    .with_scale(Vec2::splat(64.0))
                    .with_sprite(sprite)
                    .with_layer(RenderLayer::Objects)
            );
        }

        // Counter box
        if let Some(sprite) = ctx.sprite("counter_box") {
            ctx.scene.spawn(
                Entity::new(self.counter_entity)
                    .with_pos(COUNTER_POS)
                    .with_scale(Vec2::splat(120.0))
                    .with_sprite(sprite)
                    .with_layer(RenderLayer::Terrain)
            );
        }

        // Initial digit display
        self.update_counter_display(ctx);
    }

    fn update_counter_display(&mut self, ctx: &mut EngineContext) {
        // Remove old digits
        for id in self.digit_entities.drain(..) {
            ctx.scene.despawn(id);
        }

        // Format counter value
        let text = format!("{}", self.counter_value);
        let char_size = 40.0;
        let total_width = text.len() as f32 * char_size * 0.6;
        let start_x = COUNTER_POS.x - total_width / 2.0 + char_size * 0.3;

        for (i, ch) in text.chars().enumerate() {
            if let Some(digit) = ch.to_digit(10) {
                let id = self.next_entity_id();
                if let Some(mut sprite) = ctx.sprite("digits") {
                    sprite.col = digit as f32;
                    sprite.row = 0.0;
                    ctx.scene.spawn(
                        Entity::new(id)
                            .with_pos(Vec2::new(start_x + i as f32 * char_size * 0.6, COUNTER_POS.y))
                            .with_scale(Vec2::splat(char_size))
                            .with_sprite(sprite)
                            .with_layer(RenderLayer::Foreground)
                    );
                    self.digit_entities.push(id);
                }
            }
        }
    }

    fn spawn_thought_for_robot(&mut self, ctx: &mut EngineContext, robot_id: EntityId, value: i32) -> Option<EntityId> {
        let robot_pos = ctx.scene.get(robot_id)?.pos;
        let thought_pos = robot_pos + Vec2::new(0.0, -50.0);
        let id = self.next_entity_id();

        if let Some(mut sprite) = ctx.sprite("digits") {
            sprite.col = (value.abs() % 10) as f32;
            ctx.scene.spawn(
                Entity::new(id)
                    .with_pos(thought_pos)
                    .with_scale(Vec2::splat(28.0))
                    .with_sprite(sprite)
                    .with_layer(RenderLayer::UI)
            );
            // Animate pop-in
            self.tweens.add(id, Tween::scale_uniform(0.0, 28.0, 0.2, Easing::BackOut));
            Some(id)
        } else {
            None
        }
    }

    fn clear_thought(&self, ctx: &mut EngineContext, thought_id: Option<EntityId>) {
        if let Some(id) = thought_id {
            ctx.scene.despawn(id);
        }
    }

    fn move_robot_to(&mut self, ctx: &EngineContext, robot_id: EntityId, target: Vec2) {
        if let Some(entity) = ctx.scene.get(robot_id) {
            let from = entity.pos;
            self.tweens.add(robot_id, Tween::position(from, target, WALK_DURATION, Easing::QuadInOut));
        }
    }

    fn flash_counter_red(&mut self) {
        self.tweens.add(self.counter_entity,
            Tween::alpha(1.0, 0.3, 0.15, Easing::Linear)
                .with_loop(TweenLoop::PingPong)
        );
    }

    fn flash_counter_green(&mut self) {
        self.tweens.add(self.counter_entity,
            Tween::alpha(1.0, 0.5, 0.2, Easing::Linear)
                .with_loop(TweenLoop::PingPong)
        );
    }

    fn spawn_lock(&mut self, ctx: &mut EngineContext, closed: bool) {
        // Remove existing lock if any
        if let Some(id) = self.lock_entity.take() {
            ctx.scene.despawn(id);
        }

        let sprite_name = if closed { "lock_closed" } else { "lock_open" };
        if let Some(sprite) = ctx.sprite(sprite_name) {
            let id = self.next_entity_id();
            ctx.scene.spawn(
                Entity::new(id)
                    .with_pos(LOCK_POS)
                    .with_scale(Vec2::splat(48.0))
                    .with_sprite(sprite)
                    .with_layer(RenderLayer::Foreground)
            );
            self.tweens.add(id, Tween::scale_uniform(0.0, 48.0, 0.2, Easing::BackOut));
            self.lock_entity = Some(id);
            self.lock_closed = closed;
        }
    }

    fn update_lock_sprite(&mut self, ctx: &mut EngineContext, closed: bool) {
        if let Some(id) = self.lock_entity {
            let sprite_name = if closed { "lock_closed" } else { "lock_open" };
            if let Some(sprite) = ctx.sprite(sprite_name) {
                if let Some(entity) = ctx.scene.get_mut(id) {
                    entity.sprite = Some(sprite);
                }
            }
            self.lock_closed = closed;
        }
    }

    fn despawn_lock(&mut self, ctx: &mut EngineContext) {
        if let Some(id) = self.lock_entity.take() {
            ctx.scene.despawn(id);
        }
    }

    fn spawn_waiting_indicator(&mut self, ctx: &mut EngineContext, robot_id: EntityId) {
        // Clear any existing indicator
        self.despawn_waiting_indicator(ctx);

        if let Some(robot) = ctx.scene.get(robot_id) {
            let pos = robot.pos + Vec2::new(30.0, -40.0);
            if let Some(sprite) = ctx.sprite("zzz") {
                let id = self.next_entity_id();
                ctx.scene.spawn(
                    Entity::new(id)
                        .with_pos(pos)
                        .with_scale(Vec2::splat(32.0))
                        .with_sprite(sprite)
                        .with_layer(RenderLayer::UI)
                );
                self.tweens.add(id, Tween::scale_uniform(0.0, 32.0, 0.2, Easing::BackOut));
                self.waiting_indicator = Some(id);
            }
        }
    }

    fn despawn_waiting_indicator(&mut self, ctx: &mut EngineContext) {
        if let Some(id) = self.waiting_indicator.take() {
            ctx.scene.despawn(id);
        }
    }

    fn reset_scenario(&mut self, ctx: &mut EngineContext) {
        // Reset counter
        self.counter_value = 5;
        self.update_counter_display(ctx);

        // Reset robots
        self.robot_a.state = RobotState::Idle;
        self.robot_a.register = 0;
        self.robot_b.state = RobotState::Idle;
        self.robot_b.register = 0;

        // Clear thoughts
        let thought_a = self.robot_a.thought_id.take();
        let thought_b = self.robot_b.thought_id.take();
        self.clear_thought(ctx, thought_a);
        self.clear_thought(ctx, thought_b);

        // Move robots home immediately
        if let Some(e) = ctx.scene.get_mut(self.robot_a.entity_id) {
            e.pos = ROBOT_A_HOME;
        }
        if let Some(e) = ctx.scene.get_mut(self.robot_b.entity_id) {
            e.pos = ROBOT_B_HOME;
        }

        // Clear tweens
        self.tweens.clear();

        // Despawn lock and waiting indicator if exist
        self.despawn_lock(ctx);
        self.despawn_waiting_indicator(ctx);

        // Reset phases
        self.data_race_phase = DataRacePhase::Initial;
        self.mutex_phase = MutexPhase::Initial;
        self.step_timer = 0.0;
        self.playing = false;
        self.last_phase_sent = -1;

        // Send scenario info to React
        ctx.emit_event(GameEvent {
            kind: EVENT_SCENARIO,
            a: self.scenario as i32 as f32,
            b: 0.0,
            c: 0.0,
        });
    }

    fn advance_data_race(&mut self, ctx: &mut EngineContext) {
        use DataRacePhase::*;

        match self.data_race_phase {
            Initial => {
                self.data_race_phase = BothWalkToCounter;
                self.move_robot_to(ctx, self.robot_a.entity_id, COUNTER_APPROACH_A);
                self.move_robot_to(ctx, self.robot_b.entity_id, COUNTER_APPROACH_B);
                self.robot_a.state = RobotState::WalkingToCounter;
                self.robot_b.state = RobotState::WalkingToCounter;
            }
            BothWalkToCounter => {
                self.data_race_phase = BothAtCounter;
                self.robot_a.state = RobotState::AtCounter;
                self.robot_b.state = RobotState::AtCounter;
            }
            BothAtCounter => {
                self.data_race_phase = AReads;
                self.robot_a.register = self.counter_value;
                self.robot_a.state = RobotState::Reading;
                let thought = self.spawn_thought_for_robot(ctx, self.robot_a.entity_id, self.robot_a.register);
                self.robot_a.thought_id = thought;
            }
            AReads => {
                self.data_race_phase = BReads;
                self.robot_b.register = self.counter_value;
                self.robot_b.state = RobotState::Reading;
                let thought = self.spawn_thought_for_robot(ctx, self.robot_b.entity_id, self.robot_b.register);
                self.robot_b.thought_id = thought;
            }
            BReads => {
                self.data_race_phase = AIncrements;
                self.robot_a.register += 1;
                self.robot_a.state = RobotState::Incrementing;
                let old_thought = self.robot_a.thought_id.take();
                self.clear_thought(ctx, old_thought);
                let thought = self.spawn_thought_for_robot(ctx, self.robot_a.entity_id, self.robot_a.register);
                self.robot_a.thought_id = thought;
            }
            AIncrements => {
                self.data_race_phase = BIncrements;
                self.robot_b.register += 1;
                self.robot_b.state = RobotState::Incrementing;
                let old_thought = self.robot_b.thought_id.take();
                self.clear_thought(ctx, old_thought);
                let thought = self.spawn_thought_for_robot(ctx, self.robot_b.entity_id, self.robot_b.register);
                self.robot_b.thought_id = thought;
            }
            BIncrements => {
                self.data_race_phase = AWrites;
                self.counter_value = self.robot_a.register;
                self.robot_a.state = RobotState::Writing;
                self.update_counter_display(ctx);
            }
            AWrites => {
                self.data_race_phase = BWrites;
                self.counter_value = self.robot_b.register;
                self.robot_b.state = RobotState::Writing;
                self.update_counter_display(ctx);
            }
            BWrites => {
                self.data_race_phase = RaceDetected;
                self.flash_counter_red();
                ctx.emit_event(GameEvent { kind: EVENT_RACE_DETECTED, a: 1.0, b: 0.0, c: 0.0 });
            }
            RaceDetected => {
                self.data_race_phase = BothWalkHome;
                let thought_a = self.robot_a.thought_id.take();
                let thought_b = self.robot_b.thought_id.take();
                self.clear_thought(ctx, thought_a);
                self.clear_thought(ctx, thought_b);
                self.move_robot_to(ctx, self.robot_a.entity_id, ROBOT_A_HOME);
                self.move_robot_to(ctx, self.robot_b.entity_id, ROBOT_B_HOME);
                self.robot_a.state = RobotState::WalkingHome;
                self.robot_b.state = RobotState::WalkingHome;
            }
            BothWalkHome => {
                self.data_race_phase = Complete;
                self.robot_a.state = RobotState::Idle;
                self.robot_b.state = RobotState::Idle;
                self.playing = false;
            }
            Complete => {}
        }

        // Send phase to React
        let phase_num = self.data_race_phase as i32;
        if phase_num != self.last_phase_sent {
            ctx.emit_event(GameEvent {
                kind: EVENT_PHASE_NAME,
                a: phase_num as f32,
                b: 0.0,
                c: 0.0
            });
            self.last_phase_sent = phase_num;
        }
    }

    fn advance_mutex(&mut self, ctx: &mut EngineContext) {
        use MutexPhase::*;

        match self.mutex_phase {
            Initial => {
                // Spawn the lock (open initially)
                self.spawn_lock(ctx, false);
                self.mutex_phase = AWalksToCounter;
                self.move_robot_to(ctx, self.robot_a.entity_id, COUNTER_APPROACH_A);
                self.robot_a.state = RobotState::WalkingToCounter;
            }
            AWalksToCounter => {
                self.mutex_phase = AAcquiresLock;
                self.robot_a.state = RobotState::HoldingLock;
                // Close the lock - A now owns it
                self.update_lock_sprite(ctx, true);
            }
            AAcquiresLock => {
                // Robot B walks but will wait
                self.mutex_phase = AReads;
                self.move_robot_to(ctx, self.robot_b.entity_id, WAITING_POS_B);
                self.robot_b.state = RobotState::WaitingForLock;
                // Show B is waiting (ZZZ)
                self.spawn_waiting_indicator(ctx, self.robot_b.entity_id);
                // A reads the counter
                self.robot_a.register = self.counter_value;
                self.robot_a.state = RobotState::Reading;
                let thought = self.spawn_thought_for_robot(ctx, self.robot_a.entity_id, self.robot_a.register);
                self.robot_a.thought_id = thought;
            }
            AReads => {
                self.mutex_phase = AIncrements;
                self.robot_a.register += 1;
                self.robot_a.state = RobotState::Incrementing;
                let old_thought = self.robot_a.thought_id.take();
                self.clear_thought(ctx, old_thought);
                let thought = self.spawn_thought_for_robot(ctx, self.robot_a.entity_id, self.robot_a.register);
                self.robot_a.thought_id = thought;
            }
            AIncrements => {
                self.mutex_phase = AWrites;
                self.counter_value = self.robot_a.register;
                self.robot_a.state = RobotState::Writing;
                self.update_counter_display(ctx);
            }
            AWrites => {
                self.mutex_phase = AReleasesLock;
                // A releases the lock
                self.update_lock_sprite(ctx, false);
                let old_thought = self.robot_a.thought_id.take();
                self.clear_thought(ctx, old_thought);
                self.robot_a.state = RobotState::Idle;
                // A walks home
                self.move_robot_to(ctx, self.robot_a.entity_id, ROBOT_A_HOME);
            }
            AReleasesLock => {
                self.mutex_phase = BWalksToCounter;
                // B wakes up - remove waiting indicator
                self.despawn_waiting_indicator(ctx);
                // B can now approach the counter
                self.move_robot_to(ctx, self.robot_b.entity_id, COUNTER_APPROACH_B);
                self.robot_b.state = RobotState::WalkingToCounter;
            }
            BWalksToCounter => {
                self.mutex_phase = BAcquiresLock;
                self.robot_b.state = RobotState::HoldingLock;
                // Close the lock - B now owns it
                self.update_lock_sprite(ctx, true);
            }
            BAcquiresLock => {
                self.mutex_phase = BReads;
                // B reads the counter (now 6, not 5!)
                self.robot_b.register = self.counter_value;
                self.robot_b.state = RobotState::Reading;
                let thought = self.spawn_thought_for_robot(ctx, self.robot_b.entity_id, self.robot_b.register);
                self.robot_b.thought_id = thought;
            }
            BReads => {
                self.mutex_phase = BIncrements;
                self.robot_b.register += 1;
                self.robot_b.state = RobotState::Incrementing;
                let old_thought = self.robot_b.thought_id.take();
                self.clear_thought(ctx, old_thought);
                let thought = self.spawn_thought_for_robot(ctx, self.robot_b.entity_id, self.robot_b.register);
                self.robot_b.thought_id = thought;
            }
            BIncrements => {
                self.mutex_phase = BWrites;
                self.counter_value = self.robot_b.register;
                self.robot_b.state = RobotState::Writing;
                self.update_counter_display(ctx);
            }
            BWrites => {
                self.mutex_phase = BReleasesLock;
                // B releases the lock
                self.update_lock_sprite(ctx, false);
                let old_thought = self.robot_b.thought_id.take();
                self.clear_thought(ctx, old_thought);
                self.robot_b.state = RobotState::Idle;
            }
            BReleasesLock => {
                self.mutex_phase = BothWalkHome;
                // B walks home
                self.move_robot_to(ctx, self.robot_b.entity_id, ROBOT_B_HOME);
                self.robot_b.state = RobotState::WalkingHome;
                // Despawn the lock
                self.despawn_lock(ctx);
            }
            BothWalkHome => {
                self.mutex_phase = Complete;
                self.robot_a.state = RobotState::Idle;
                self.robot_b.state = RobotState::Idle;
                self.playing = false;
                // Success! Counter is now 7 as expected
                self.flash_counter_green();
                ctx.emit_event(GameEvent { kind: EVENT_SUCCESS, a: 1.0, b: 0.0, c: 0.0 });
            }
            Complete => {}
        }

        // Send phase to React (offset by 100 to distinguish from data race phases)
        let phase_num = self.mutex_phase as i32 + 100;
        if phase_num != self.last_phase_sent {
            ctx.emit_event(GameEvent {
                kind: EVENT_PHASE_NAME,
                a: phase_num as f32,
                b: 0.0,
                c: 0.0
            });
            self.last_phase_sent = phase_num;
        }
    }

    fn handle_custom_event(&mut self, kind: u32, a: f32, b: f32, ctx: &mut EngineContext) {
        match kind {
            CUSTOM_PLAY => self.playing = true,
            CUSTOM_PAUSE => self.playing = false,
            CUSTOM_STEP => {
                match self.scenario {
                    Scenario::DataRace => self.advance_data_race(ctx),
                    Scenario::MutexFix => self.advance_mutex(ctx),
                }
            }
            CUSTOM_RESET => self.reset_scenario(ctx),
            CUSTOM_SCENARIO => {
                self.scenario = match a as u32 {
                    0 => Scenario::DataRace,
                    1 => Scenario::MutexFix,
                    _ => Scenario::DataRace,
                };
                self.reset_scenario(ctx);
            }
            99 => {
                self.visible_w = a;
                self.visible_h = b;
            }
            _ => {}
        }
    }

    fn update_thought_positions(&self, ctx: &mut EngineContext) {
        // Robot A thought bubble
        if let Some(thought_id) = self.robot_a.thought_id {
            if let Some(robot) = ctx.scene.get(self.robot_a.entity_id) {
                let target_pos = robot.pos + Vec2::new(0.0, -50.0);
                if let Some(thought) = ctx.scene.get_mut(thought_id) {
                    thought.pos = target_pos;
                }
            }
        }

        // Robot B thought bubble
        if let Some(thought_id) = self.robot_b.thought_id {
            if let Some(robot) = ctx.scene.get(self.robot_b.entity_id) {
                let target_pos = robot.pos + Vec2::new(0.0, -50.0);
                if let Some(thought) = ctx.scene.get_mut(thought_id) {
                    thought.pos = target_pos;
                }
            }
        }

        // Waiting indicator (follows Robot B)
        if let Some(indicator_id) = self.waiting_indicator {
            if let Some(robot) = ctx.scene.get(self.robot_b.entity_id) {
                let target_pos = robot.pos + Vec2::new(30.0, -40.0);
                if let Some(indicator) = ctx.scene.get_mut(indicator_id) {
                    indicator.pos = target_pos;
                }
            }
        }
    }
}

impl Game for ThreadTownGame {
    fn config(&self) -> GameConfig {
        GameConfig {
            world_width: WORLD_W,
            world_height: WORLD_H,
            fixed_dt: FIXED_DT,
            ..Default::default()
        }
    }

    fn init(&mut self, _ctx: &mut EngineContext) {
        // Setup deferred to first update when sprites are available
    }

    fn update(&mut self, ctx: &mut EngineContext, input: &InputQueue) {
        let dt = FIXED_DT;

        // Handle input events
        for event in input.iter() {
            if let InputEvent::Custom { kind, a, b, .. } = event {
                self.handle_custom_event(*kind, *a, *b, ctx);
            }
        }

        // Initialize scene once sprites are loaded
        if !self.initialized {
            self.setup_scene(ctx);
            self.initialized = true;
        }

        // Update tweens
        self.tweens.tick(dt, &mut ctx.scene);

        // Update thought bubble positions
        self.update_thought_positions(ctx);

        // Auto-advance if playing
        if self.playing {
            self.step_timer += dt;
            if self.step_timer >= STEP_DURATION {
                self.step_timer = 0.0;
                match self.scenario {
                    Scenario::DataRace => self.advance_data_race(ctx),
                    Scenario::MutexFix => self.advance_mutex(ctx),
                }
            }
        }

        // Send counter value to React
        ctx.emit_event(GameEvent {
            kind: EVENT_COUNTER_VALUE,
            a: self.counter_value as f32,
            b: 0.0,
            c: 0.0,
        });
    }
}
