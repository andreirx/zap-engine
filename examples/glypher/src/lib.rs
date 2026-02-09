use wasm_bindgen::prelude::*;
use zap_engine::*;

mod game;
mod glyphs;
mod sayings;
mod tracing;
use game::Glypher;

zap_web::export_game!(Glypher, "glypher", vectors);
