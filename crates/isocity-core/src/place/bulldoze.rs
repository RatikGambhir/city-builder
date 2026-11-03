//! Bulldozing tiles, including whole bridges and multi-tile buildings.

use crate::buildings::BuildingType;
use crate::sim::find_building_origin;
use crate::types::*;

fn connected_bridge_tiles(grid: &Grid, x: i32, y: i32) -> Vec<(i32, i32)> {
    let Some(t) = grid.get(x, y) else {
        return vec![];
    };
    if t.building.kind != BuildingType::Bridge {
        return vec![];
    }
    let o = t.building.bridge_orientation.unwrap_or(Orientation::Ns);
    let (dx, dy) = if o == Orientation::Ns { (1, 0) } else { (0, 1) };
    let mut out = vec![(x, y)];
    for sign in [1, -1] {
        let (mut cx, mut cy) = (x + dx * sign, y + dy * sign);
        while let Some(n) = grid.get(cx, cy) {
            if n.building.kind == BuildingType::Bridge && n.building.bridge_orientation == Some(o) {
                out.push((cx, cy));
                cx += dx * sign;
                cy += dy * sign;
            } else {
                break;
            }
        }
    }
    out
}

fn adjacent_bridge_tiles(grid: &Grid, x: i32, y: i32) -> Vec<(i32, i32)> {
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        if let Some(n) = grid.get(x + dx, y + dy) {
            if n.building.kind == BuildingType::Bridge
                && matches!(n.building.bridge_position, Some(BridgePosition::Start) | Some(BridgePosition::End))
            {
                return connected_bridge_tiles(grid, x + dx, y + dy);
            }
        }
    }
    vec![]
}

pub(super) fn clear_tile(t: &mut Tile, kind: BuildingType) {
    t.building = Building::new(kind);
    t.zone = ZoneType::None;
    t.has_rail_overlay = false;
}

pub fn bulldoze(state: &mut GameState, x: i32, y: i32) -> bool {
    let Some(tile) = state.grid.get(x, y) else {
        return false;
    };
    let kind = tile.building.kind;
    if kind == BuildingType::Water || (kind == BuildingType::Grass && tile.zone == ZoneType::None) {
        return false;
    }
    if kind == BuildingType::Bridge {
        for (bx, by) in connected_bridge_tiles(&state.grid, x, y) {
            clear_tile(state.grid.at_mut(bx, by), BuildingType::Water);
        }
    } else if kind == BuildingType::Road && !adjacent_bridge_tiles(&state.grid, x, y).is_empty() {
        let bridge = adjacent_bridge_tiles(&state.grid, x, y);
        clear_tile(state.grid.at_mut(x, y), BuildingType::Grass);
        for (bx, by) in bridge {
            clear_tile(state.grid.at_mut(bx, by), BuildingType::Water);
        }
    } else if let Some((ox, oy, k)) = find_building_origin(&state.grid, x, y) {
        let fp = k.size();
        for dy in 0..fp.height {
            for dx in 0..fp.width {
                if let Some(t) = state.grid.get_mut(ox + dx, oy + dy) {
                    clear_tile(t, BuildingType::Grass);
                }
            }
        }
    } else {
        clear_tile(state.grid.at_mut(x, y), BuildingType::Grass);
    }
    state.touch();
    true
}
