//! Thread Town — A synchronization visualizer for teaching concurrency.
//!
//! Visual metaphors:
//! - Robots = Threads
//! - Key = Mutex
//! - Counter = Shared memory
//! - Waiting room = Condition variable
//! - Bell = notify_one/notify_all

use wasm_bindgen::prelude::*;
use zap_engine::*;

mod game;

use game::ThreadTownGame;

// Export all WASM bindings via macro
zap_web::export_game!(ThreadTownGame, "thread-town");
