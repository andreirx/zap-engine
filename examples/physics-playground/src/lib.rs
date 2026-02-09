use wasm_bindgen::prelude::*;
use zap_engine::*;

mod game;
use game::PhysicsPlayground;

zap_web::export_game!(PhysicsPlayground, "physics-playground", vectors);
