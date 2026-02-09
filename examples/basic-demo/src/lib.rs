use wasm_bindgen::prelude::*;
use zap_engine::*;

mod game;
use game::BasicDemo;

zap_web::export_game!(BasicDemo, "basic-demo", vectors);
