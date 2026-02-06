use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use zap_engine::*;
use zap_web::GameRunner;

mod game;
use game::PhysicsPlayground;

thread_local! {
    static RUNNER: RefCell<Option<GameRunner<PhysicsPlayground>>> = RefCell::new(None);
}

fn with_runner<R>(f: impl FnOnce(&mut GameRunner<PhysicsPlayground>) -> R) -> R {
    RUNNER.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let runner = borrow.as_mut().expect("Game not initialized. Call game_init() first.");
        f(runner)
    })
}

#[wasm_bindgen]
pub fn game_init() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);

    let game = PhysicsPlayground::new();
    let runner = GameRunner::new(game);

    RUNNER.with(|cell| {
        *cell.borrow_mut() = Some(runner);
    });

    with_runner(|r| r.init());
    log::info!("physics-playground: initialized");
}

#[wasm_bindgen]
pub fn game_tick(dt: f32) {
    with_runner(|r| r.tick(dt));
}

#[wasm_bindgen]
pub fn game_pointer_down(x: f32, y: f32) {
    with_runner(|r| r.push_input(InputEvent::PointerDown { x, y }));
}

#[wasm_bindgen]
pub fn game_pointer_up(x: f32, y: f32) {
    with_runner(|r| r.push_input(InputEvent::PointerUp { x, y }));
}

#[wasm_bindgen]
pub fn game_pointer_move(x: f32, y: f32) {
    with_runner(|r| r.push_input(InputEvent::PointerMove { x, y }));
}

#[wasm_bindgen]
pub fn game_key_down(key_code: u32) {
    with_runner(|r| r.push_input(InputEvent::KeyDown { key_code }));
}

#[wasm_bindgen]
pub fn game_key_up(key_code: u32) {
    with_runner(|r| r.push_input(InputEvent::KeyUp { key_code }));
}

#[wasm_bindgen]
pub fn game_custom_event(kind: u32, a: f32, b: f32, c: f32) {
    with_runner(|r| r.push_input(InputEvent::Custom { kind, a, b, c }));
}

#[wasm_bindgen]
pub fn game_load_manifest(json: &str) {
    with_runner(|r| r.load_manifest(json));
}

#[wasm_bindgen]
pub fn get_instances_ptr() -> *const f32 {
    with_runner(|r| r.instances_ptr())
}

#[wasm_bindgen]
pub fn get_instance_count() -> u32 {
    with_runner(|r| r.instance_count())
}

#[wasm_bindgen]
pub fn get_effects_ptr() -> *const f32 {
    with_runner(|r| r.effects_ptr())
}

#[wasm_bindgen]
pub fn get_effects_vertex_count() -> u32 {
    with_runner(|r| r.effects_vertex_count())
}

#[wasm_bindgen]
pub fn get_sound_events_ptr() -> *const u8 {
    with_runner(|r| r.sound_events_ptr())
}

#[wasm_bindgen]
pub fn get_sound_events_len() -> u32 {
    with_runner(|r| r.sound_events_len())
}

#[wasm_bindgen]
pub fn get_game_events_ptr() -> *const f32 {
    with_runner(|r| r.game_events_ptr())
}

#[wasm_bindgen]
pub fn get_game_events_len() -> u32 {
    with_runner(|r| r.game_events_len())
}

#[wasm_bindgen]
pub fn get_world_width() -> f32 {
    with_runner(|r| r.world_width())
}

#[wasm_bindgen]
pub fn get_world_height() -> f32 {
    with_runner(|r| r.world_height())
}

#[wasm_bindgen]
pub fn get_atlas_split() -> u32 {
    with_runner(|r| r.atlas_split())
}

#[wasm_bindgen]
pub fn get_max_instances() -> u32 {
    with_runner(|r| r.max_instances())
}

#[wasm_bindgen]
pub fn get_max_effects_vertices() -> u32 {
    with_runner(|r| r.max_effects_vertices())
}

#[wasm_bindgen]
pub fn get_max_sounds() -> u32 {
    with_runner(|r| r.max_sounds())
}

#[wasm_bindgen]
pub fn get_max_events() -> u32 {
    with_runner(|r| r.max_events())
}

#[wasm_bindgen]
pub fn get_buffer_total_floats() -> u32 {
    with_runner(|r| r.buffer_total_floats())
}

#[wasm_bindgen]
pub fn get_sdf_instances_ptr() -> *const f32 {
    with_runner(|r| r.sdf_instances_ptr())
}

#[wasm_bindgen]
pub fn get_sdf_instance_count() -> u32 {
    with_runner(|r| r.sdf_instance_count())
}

#[wasm_bindgen]
pub fn get_max_sdf_instances() -> u32 {
    with_runner(|r| r.max_sdf_instances())
}

#[cfg(feature = "vectors")]
#[wasm_bindgen]
pub fn get_vector_vertices_ptr() -> *const f32 {
    with_runner(|r| r.vector_vertices_ptr())
}

#[cfg(feature = "vectors")]
#[wasm_bindgen]
pub fn get_vector_vertex_count() -> u32 {
    with_runner(|r| r.vector_vertex_count())
}

#[cfg(feature = "vectors")]
#[wasm_bindgen]
pub fn get_max_vector_vertices() -> u32 {
    with_runner(|r| r.max_vector_vertices())
}
