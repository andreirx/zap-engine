use wasm_bindgen::prelude::*;
use zap_engine::*;

mod flags;
mod game;
use game::FlagParade;

zap_web::export_game!(FlagParade, "flag-parade", vectors);
