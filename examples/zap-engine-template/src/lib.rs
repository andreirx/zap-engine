use wasm_bindgen::prelude::*;
use zap_engine::*;

mod game;
use game::HelloGame;

zap_web::export_game!(HelloGame, "zap-engine-template", vectors);
