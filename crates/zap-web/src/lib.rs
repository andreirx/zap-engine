pub mod runner;

pub use runner::GameRunner;

/// Generate all `#[wasm_bindgen]` exports for a game.
///
/// This macro eliminates ~230 lines of boilerplate per game by generating:
/// - `thread_local!` storage for the GameRunner
/// - `with_runner()` helper function
/// - All wasm-bindgen exports (game_init, game_tick, input handlers, data accessors)
///
/// # Usage
///
/// ```ignore
/// use wasm_bindgen::prelude::*;
/// use zap_engine::*;
/// use zap_web::GameRunner;
///
/// mod game;
/// use game::MyGame;
///
/// zap_web::export_game!(MyGame, "my-game");
/// ```
///
/// # Arguments
///
/// - `$game_type`: The game struct type that implements `zap_engine::Game`
/// - `$game_name`: A string literal used in the initialization log message
#[macro_export]
macro_rules! export_game {
    ($game_type:ty, $game_name:literal) => {
        use std::cell::RefCell;

        thread_local! {
            static RUNNER: RefCell<Option<$crate::GameRunner<$game_type>>> = RefCell::new(None);
        }

        fn with_runner<R>(f: impl FnOnce(&mut $crate::GameRunner<$game_type>) -> R) -> R {
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

            let game = <$game_type>::new();
            let runner = $crate::GameRunner::new(game);

            RUNNER.with(|cell| {
                *cell.borrow_mut() = Some(runner);
            });

            with_runner(|r| r.init());
            log::info!("{}: initialized", $game_name);
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

        // ---- Data accessors ----

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

        // ---- Capacity accessors ----

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

        // ---- SDF accessors ----

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

        // ---- Layer batch accessors ----

        #[wasm_bindgen]
        pub fn get_layer_batches_ptr() -> *const f32 {
            with_runner(|r| r.layer_batches_ptr())
        }

        #[wasm_bindgen]
        pub fn get_layer_batch_count() -> u32 {
            with_runner(|r| r.layer_batch_count())
        }

        #[wasm_bindgen]
        pub fn get_max_layer_batches() -> u32 {
            with_runner(|r| r.max_layer_batches())
        }

        #[wasm_bindgen]
        pub fn get_layer_batch_data_offset() -> u32 {
            with_runner(|r| r.layer_batch_data_offset())
        }

        // ---- Bake state accessor ----

        #[wasm_bindgen]
        pub fn get_bake_state() -> f32 {
            with_runner(|r| r.bake_state())
        }

        // ---- Lighting accessors ----

        #[wasm_bindgen]
        pub fn get_lights_ptr() -> *const f32 {
            with_runner(|r| r.lights_ptr())
        }

        #[wasm_bindgen]
        pub fn get_light_count() -> u32 {
            with_runner(|r| r.light_count())
        }

        #[wasm_bindgen]
        pub fn get_max_lights() -> u32 {
            with_runner(|r| r.max_lights())
        }

        #[wasm_bindgen]
        pub fn get_ambient_r() -> f32 {
            with_runner(|r| r.ambient_r())
        }

        #[wasm_bindgen]
        pub fn get_ambient_g() -> f32 {
            with_runner(|r| r.ambient_g())
        }

        #[wasm_bindgen]
        pub fn get_ambient_b() -> f32 {
            with_runner(|r| r.ambient_b())
        }
    };

    // Variant with vectors feature
    ($game_type:ty, $game_name:literal, vectors) => {
        $crate::export_game!($game_type, $game_name);

        // ---- Vector accessors (only when vectors feature is enabled) ----

        #[wasm_bindgen]
        pub fn get_vector_vertices_ptr() -> *const f32 {
            with_runner(|r| r.vector_vertices_ptr())
        }

        #[wasm_bindgen]
        pub fn get_vector_vertex_count() -> u32 {
            with_runner(|r| r.vector_vertex_count())
        }

        #[wasm_bindgen]
        pub fn get_max_vector_vertices() -> u32 {
            with_runner(|r| r.max_vector_vertices())
        }
    };
}
