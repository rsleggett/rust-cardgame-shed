//! Library crate exposing the game modules so integration tests in `tests/`
//! can drive Bevy `App`s against the same code the binary runs.

pub mod ai;
pub mod audio;
pub mod components;
pub mod game_plugin;
pub mod rendering;
pub mod rules;
pub mod sfx;
pub mod theme;
pub mod systems;
pub mod ui;
