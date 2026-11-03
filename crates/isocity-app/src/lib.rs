//! IsoCity desktop: an isometric city builder built with GPUI.
//!
//! The library holds the game's modules so integration tests under `tests/`
//! can reach them; `main.rs` only parses arguments and opens the window.

pub mod assets;
pub mod entities;
pub mod icons;
pub mod iso;
pub mod render;
pub mod sprites;
pub mod theme;
pub mod ui;
pub mod world;
