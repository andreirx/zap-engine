use wasm_bindgen::prelude::*;
use zap_engine::*;

mod animation;
mod board;
mod game;
use game::ZapZapMini;

zap_web::export_game!(ZapZapMini, "zapzap-mini", vectors);
