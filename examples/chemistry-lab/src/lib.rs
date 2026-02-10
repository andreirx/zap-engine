use wasm_bindgen::prelude::*;
use zap_engine::*;

// Core modules
mod math3d;
mod periodic_table;
mod vsepr;

// Simulation modules
mod molecule3d;
mod physics;
mod sim;

// Interaction and rendering
mod interaction;
mod render3d;
mod renderer;

// Legacy modules (kept for reference/tests)
mod bohr;
mod chemistry;
mod elements;
mod molecule;

// Main game controller
mod game;

use game::ChemistryLab;

zap_web::export_game!(ChemistryLab, "chemistry-lab", vectors);
