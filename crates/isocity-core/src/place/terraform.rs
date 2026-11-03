//! Turning land into water and back.

use crate::buildings::BuildingType;
use crate::sim::find_building_origin;
use crate::types::*;
use super::bulldoze::clear_tile;

pub fn terraform_water(state: &mut GameState, x: i32, y: i32) -> bool {
    let Some(t) = state.grid.get(x, y) else {
        return false;
    };
    if matches!(t.building.kind, BuildingType::Water | BuildingType::Bridge) {
        return false;
    }
    if let Some((ox, oy, k)) = find_building_origin(&state.grid, x, y) {
        let fp = k.size();
        for dy in 0..fp.height {
            for dx in 0..fp.width {
                if let Some(t) = state.grid.get_mut(ox + dx, oy + dy) {
                    t.building = Building::new(BuildingType::Grass);
                    t.zone = ZoneType::None;
                }
            }
        }
    }
    let t = state.grid.at_mut(x, y);
    clear_tile(t, BuildingType::Water);
    t.has_subway = false;
    state.touch();
    true
}

pub fn terraform_land(state: &mut GameState, x: i32, y: i32) -> bool {
    match state.grid.get_mut(x, y) {
        Some(t) if t.building.kind == BuildingType::Water => {
            clear_tile(t, BuildingType::Grass);
            state.touch();
            true
        }
        _ => false,
    }
}
