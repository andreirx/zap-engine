use wasm_bindgen::prelude::*;
use zap_engine::*;

mod bodies;
mod game;
mod orbit;
use game::SolarSystem;

zap_web::export_game!(SolarSystem, "solar-system", vectors);
