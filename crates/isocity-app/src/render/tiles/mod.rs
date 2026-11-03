//! Static ground layer painting (everything the web version draws on its main
//! canvas): base tiles, zone tints, water with beaches, roads with markings,
//! rail tracks, bridges, building bases and construction plots.

mod map;
mod road_merge;
mod palette;
mod ground;
mod road;
mod rail;
mod bridge;

pub use map::TileMap;
pub use ground::{draw_base_tile, draw_beach_on_water, draw_foundation, draw_green_base, draw_grey_base, draw_water, draw_water_under};
pub use road::draw_road;
pub use rail::{draw_rail_tile, draw_rail_track};
pub use bridge::draw_bridge;

use super::canvas2d::P;
use crate::iso::{TILE_H, TILE_W};
use tiny_skia::Pixmap;

fn diamond(x: f32, y: f32) -> [P; 4] {
    [
        (x + TILE_W / 2.0, y),
        (x + TILE_W, y + TILE_H / 2.0),
        (x + TILE_W / 2.0, y + TILE_H),
        (x, y + TILE_H / 2.0),
    ]
}

fn norm(dx: f32, dy: f32) -> P {
    let l = (dx * dx + dy * dy).sqrt().max(1e-6);
    (dx / l, dy / l)
}

/// Painting parameters shared by a chunk bake.
pub struct PaintCtx<'a> {
    pub map: &'a TileMap,
    pub zoom: f32,
    pub water: Option<&'a Pixmap>,
}
