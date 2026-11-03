//! Player edits to the grid: placing buildings and zones, bulldozing,
//! bridges, subways, terraforming and growing/shrinking the map.

mod bulldoze;
mod terraform;
mod bridges;
mod resize;

pub use bulldoze::bulldoze;
pub use terraform::{terraform_land, terraform_water};
pub use bridges::create_bridges_on_path;
pub use resize::{expand_grid, has_road_at_edge, shrink_grid};

use crate::buildings::BuildingType;
use crate::sim::{apply_building_footprint, calculate_service_coverage, find_building_origin};
use crate::types::*;

pub const MAX_BRIDGE_SPAN: usize = 10;

fn can_place_multi(grid: &Grid, x: i32, y: i32, w: i32, h: i32) -> bool {
    if x < 0 || y < 0 || x + w > grid.size() || y + h > grid.size() {
        return false;
    }
    (0..h).all(|dy| {
        (0..w).all(|dx| {
            matches!(
                grid.at(x + dx, y + dy).building.kind,
                BuildingType::Grass | BuildingType::Tree
            )
        })
    })
}

/// `(has_water, should_flip)` for a waterfront footprint.
pub fn water_adjacency(grid: &Grid, x: i32, y: i32, w: i32, h: i32) -> (bool, bool) {
    let front = (0..w).any(|dx| grid.is_water(x + dx, y + h)) || (0..h).any(|dy| grid.is_water(x + w, y + dy));
    let back = (0..w).any(|dx| grid.is_water(x + dx, y - 1)) || (0..h).any(|dy| grid.is_water(x - 1, y + dy));
    let has = front || back;
    (has, has && back && !front)
}

/// `(has_road, should_flip)` so building fronts face the street.
pub fn road_adjacency(grid: &Grid, x: i32, y: i32, w: i32, h: i32) -> (bool, bool) {
    let front = (0..w).any(|dx| grid.is_road_or_bridge(x + dx, y + h))
        || (0..h).any(|dy| grid.is_road_or_bridge(x + w, y + dy));
    let back = (0..w).any(|dx| grid.is_road_or_bridge(x + dx, y - 1))
        || (0..h).any(|dy| grid.is_road_or_bridge(x - 1, y + dy));
    let has = front || back;
    (has, has && back && !front)
}

/// Places a building or paints a zone. Returns `true` when the grid changed.
pub fn place_building(
    state: &mut GameState,
    x: i32,
    y: i32,
    building: Option<BuildingType>,
    zone: Option<ZoneType>,
) -> bool {
    let Some(tile) = state.grid.get(x, y) else {
        return false;
    };
    let cur = tile.building.kind;
    if cur == BuildingType::Water {
        return false;
    }
    let has_overlay = tile.has_rail_overlay;
    let cur_zone = tile.zone;
    if let Some(b) = building {
        let track = matches!(b, BuildingType::Road | BuildingType::Rail);
        if track && !matches!(cur, BuildingType::Grass | BuildingType::Tree | BuildingType::Road | BuildingType::Rail) {
            return false;
        }
        if !track && matches!(cur, BuildingType::Road | BuildingType::Bridge | BuildingType::Rail) {
            return false;
        }
    }

    if let Some(zone) = zone {
        if zone == ZoneType::None {
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
            } else {
                if cur_zone == ZoneType::None {
                    return false;
                }
                let t = state.grid.at_mut(x, y);
                t.zone = ZoneType::None;
                t.building = Building::new(BuildingType::Grass);
            }
        } else {
            if !matches!(cur, BuildingType::Grass | BuildingType::Tree | BuildingType::Road) {
                return false;
            }
            state.grid.at_mut(x, y).zone = zone;
        }
        state.touch();
        return true;
    }

    let Some(kind) = building else {
        return false;
    };
    let fp = kind.size();
    let mut flip = false;
    if kind.requires_water_adjacency() {
        let (has, f) = water_adjacency(&state.grid, x, y, fp.width, fp.height);
        if !has {
            return false;
        }
        flip = f;
    }
    if fp.is_multi() {
        if !can_place_multi(&state.grid, x, y, fp.width, fp.height) {
            return false;
        }
        apply_building_footprint(&mut state.grid, x, y, kind, ZoneType::None, 1, None);
        state.grid.at_mut(x, y).building.flipped = flip;
    } else {
        if !matches!(cur, BuildingType::Grass | BuildingType::Tree | BuildingType::Road | BuildingType::Rail) {
            return false;
        }
        let t = state.grid.at_mut(x, y);
        match (kind, cur) {
            (BuildingType::Rail, BuildingType::Road) => {
                if has_overlay {
                    return false;
                }
                t.has_rail_overlay = true;
            }
            (BuildingType::Road, BuildingType::Rail) => {
                t.building = Building::new(BuildingType::Road);
                t.has_rail_overlay = true;
                t.zone = ZoneType::None;
            }
            _ => {
                if kind == cur {
                    return false;
                }
                t.building = Building::new(kind);
                t.zone = ZoneType::None;
                if kind != BuildingType::Road {
                    t.has_rail_overlay = false;
                }
            }
        }
        t.building.flipped = flip;
    }
    state.touch();
    true
}

pub fn place_subway(state: &mut GameState, x: i32, y: i32) -> bool {
    match state.grid.get_mut(x, y) {
        Some(t) if t.building.kind != BuildingType::Water && !t.has_subway => {
            t.has_subway = true;
            true
        }
        _ => false,
    }
}

/// Raises a service building's level, extending its coverage radius.
pub fn upgrade_service(state: &mut GameState, x: i32, y: i32) -> bool {
    let Some(t) = state.grid.get(x, y) else {
        return false;
    };
    let b = &t.building;
    if !b.kind.is_service() || b.level >= crate::sim::SERVICE_MAX_LEVEL || b.under_construction() || b.abandoned {
        return false;
    }
    let Some(cost) = crate::sim::upgrade_cost(b.kind, b.level) else {
        return false;
    };
    if state.stats.money < cost {
        return false;
    }
    state.grid.at_mut(x, y).building.level += 1;
    state.stats.money -= cost;
    state.services = calculate_service_coverage(&state.grid);
    state.touch();
    true
}
