//! Growing and shrinking the map.

use crate::buildings::BuildingType;
use crate::sim::{calculate_service_coverage, find_building_origin};
use crate::terrain;
use crate::types::*;

pub fn has_road_at_edge(grid: &Grid, dir: Direction) -> bool {
    let n = grid.size();
    (0..n).any(|i| {
        let (x, y) = match dir {
            Direction::North => (i, 0),
            Direction::South => (i, n - 1),
            Direction::East => (n - 1, i),
            Direction::West => (0, i),
        };
        grid.is_road_or_bridge(x, y)
    })
}

/// Grows the map by `expansion` tiles on every side, continuing water and
/// land naturally from the old border.
pub fn expand_grid(state: &mut GameState, expansion: i32) {
    let old = state.grid.clone();
    let old_size = old.size();
    let new_size = old_size + expansion * 2;
    let seed = state.rng.next_f64() * 1000.0;
    let mut grid = Grid::new(new_size, |x, y| {
        let (ox, oy) = (x - expansion, y - expansion);
        if let Some(t) = old.get(ox, oy) {
            let mut t = t.clone();
            t.x = x;
            t.y = y;
            return t;
        }
        let cx = ox.clamp(0, old_size - 1);
        let cy = oy.clamp(0, old_size - 1);
        let dist = (ox - cx).abs().max((oy - cy).abs());
        let edge_water = old.is_water(cx, cy);
        let n = terrain::perlin(x as f64, y as f64, seed, 3);
        let kind = if edge_water && (n > 0.35 || dist < 4) {
            BuildingType::Water
        } else {
            BuildingType::Grass
        };
        let mut t = Tile::new(x, y, kind);
        if kind == BuildingType::Water {
            t.land_value = 60.0;
        }
        t
    });
    let mut rng = std::mem::take(&mut state.rng);
    for y in 0..new_size {
        for x in 0..new_size {
            let inside = x >= expansion && y >= expansion && x < expansion + old_size && y < expansion + old_size;
            if inside || grid.is_water(x, y) {
                continue;
            }
            let tn = terrain::perlin(x as f64 * 2.0, y as f64 * 2.0, seed + 500.0, 2);
            if tn > 0.72 && rng.next_f64() > 0.65 {
                grid.at_mut(x, y).building = Building::new(BuildingType::Tree);
            }
        }
    }
    state.rng = rng;
    for wb in &mut state.water_bodies {
        for p in &mut wb.tiles {
            p.x += expansion;
            p.y += expansion;
        }
        wb.center_x += expansion as f32;
        wb.center_y += expansion as f32;
    }
    state.grid = grid;
    state.grid_size = new_size;
    state.services = calculate_service_coverage(&state.grid);
    state.touch();
}

/// Removes `amount` tiles from every side (never below 20x20).
pub fn shrink_grid(state: &mut GameState, amount: i32) -> bool {
    let old = state.grid.clone();
    let new_size = old.size() - amount * 2;
    if new_size < 20 {
        return false;
    }
    let mut grid = Grid::new(new_size, |x, y| {
        let mut t = old.at(x + amount, y + amount).clone();
        t.x = x;
        t.y = y;
        t
    });
    // Remove multi-tile buildings that were cut.
    for y in 0..new_size {
        for x in 0..new_size {
            if grid.at(x, y).building.kind == BuildingType::Empty && find_building_origin(&grid, x, y).is_none() {
                grid.at_mut(x, y).building = Building::new(BuildingType::Grass);
            }
            let k = grid.at(x, y).building.kind;
            let fp = k.size();
            if fp.is_multi() && (x + fp.width > new_size || y + fp.height > new_size) {
                for dy in 0..fp.height {
                    for dx in 0..fp.width {
                        if let Some(t) = grid.get_mut(x + dx, y + dy) {
                            t.building = Building::new(BuildingType::Grass);
                        }
                    }
                }
            }
        }
    }
    for wb in &mut state.water_bodies {
        wb.tiles.retain_mut(|p| {
            p.x -= amount;
            p.y -= amount;
            p.x >= 0 && p.y >= 0 && p.x < new_size && p.y < new_size
        });
        wb.center_x -= amount as f32;
        wb.center_y -= amount as f32;
    }
    state.water_bodies.retain(|w| !w.tiles.is_empty());
    state.grid = grid;
    state.grid_size = new_size;
    state.services = calculate_service_coverage(&state.grid);
    state.touch();
    true
}
