use wasm_bindgen::prelude::*;
use zap_engine::*;

mod game;
use game::DinoLevelEditor;

zap_web::export_game!(DinoLevelEditor, "dino-level-editor", vectors);

#[wasm_bindgen]
pub fn request_world_export() {}

#[wasm_bindgen]
pub fn take_world_export() -> Option<String> {
    Some(with_runner(|runner| runner.game().export_level_json()))
}

#[wasm_bindgen]
pub fn load_level(json: &str) {
    with_runner(|runner| {
        if let Err(err) = runner.game_mut().load_level_json(json) {
            web_sys::console::error_1(&format!("Failed to load dino level: {err}").into());
        }
    });
}
