use wasm_bindgen::prelude::*;
use zap_engine::*;

mod elements;
mod game;
mod molecule;
use game::ChemistryLab;

zap_web::export_game!(ChemistryLab, "chemistry-lab", vectors);
