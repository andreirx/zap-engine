use wasm_bindgen::prelude::*;
use zap_engine::*;

mod balls;
mod game;

use game::PoolGame;

zap_web::export_game!(PoolGame, "pool-game", vectors);
