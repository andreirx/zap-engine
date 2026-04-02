use wasm_bindgen::prelude::*;
use zap_engine::*;

mod game;
use game::EffectsShowcase;

zap_web::export_game!(EffectsShowcase, "effects-showcase", vectors);
