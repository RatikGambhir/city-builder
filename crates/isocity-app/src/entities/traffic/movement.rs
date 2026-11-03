//! Direction vectors, lanes and route choice shared by all vehicles.

use crate::entities::paths;
use crate::iso::{tile_center, TILE_H, TILE_W};
use isocity_core::{BuildingType as B, Direction, Grid, Rng};

/// Screen vector for one tile step in `dir`.
pub fn dir_vec(dir: Direction) -> (f32, f32) {
    match dir {
        Direction::North => (-TILE_W / 2.0, -TILE_H / 2.0),
        Direction::East => (TILE_W / 2.0, -TILE_H / 2.0),
        Direction::South => (TILE_W / 2.0, TILE_H / 2.0),
        Direction::West => (-TILE_W / 2.0, TILE_H / 2.0),
    }
}

pub fn dir_angle(dir: Direction) -> f32 {
    let (dx, dy) = dir_vec(dir);
    dy.atan2(dx)
}

pub fn dir_normal(dir: Direction) -> (f32, f32) {
    let (dx, dy) = dir_vec(dir);
    let l = (dx * dx + dy * dy).sqrt();
    (-dy / l, dx / l)
}

pub(super) fn lane_offset(dir: Direction, base: f32) -> f32 {
    if matches!(dir, Direction::North | Direction::East) {
        base
    } else {
        -base
    }
}

/// World position of a vehicle on a tile.
pub fn vehicle_pos(x: i32, y: i32, dir: Direction, progress: f32, lane: f32) -> (f32, f32) {
    let (cx, cy) = tile_center(x as f32, y as f32);
    let (vx, vy) = dir_vec(dir);
    let (nx, ny) = dir_normal(dir);
    (cx + vx * progress + nx * lane, cy + vy * progress + ny * lane)
}

pub fn speed_multiplier(speed: u8) -> f32 {
    match speed {
        0 => 0.0,
        1 => 1.0,
        2 => 2.5,
        _ => 4.0,
    }
}

pub(super) fn pick_next(grid: &Grid, rng: &mut Rng, prev: Direction, x: i32, y: i32) -> Option<Direction> {
    let opts = paths::road_options(grid, x, y);
    if opts.is_empty() {
        return None;
    }
    if grid.at(x, y).building.kind == B::Bridge && opts.contains(&prev) {
        return Some(prev);
    }
    let back = prev.opposite();
    let forward: Vec<Direction> = opts.iter().copied().filter(|d| *d != back).collect();
    let pool = if forward.is_empty() { &opts } else { &forward };
    rng.pick(pool).copied()
}
