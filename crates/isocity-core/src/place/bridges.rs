//! Automatic bridges where a dragged road or rail crosses water.

use crate::buildings::BuildingType;
use crate::types::*;
use super::MAX_BRIDGE_SPAN;

fn bridge_type_for_span(span: usize) -> BridgeType {
    if span == 1 {
        BridgeType::Small
    } else if span <= 5 {
        BridgeType::Large
    } else {
        BridgeType::Suspension
    }
}

fn bridge_variants(t: BridgeType) -> i32 {
    match t {
        BridgeType::Small | BridgeType::Medium => 3,
        BridgeType::Large | BridgeType::Suspension => 2,
    }
}

struct BridgeOpportunity {
    orientation: Orientation,
    bridge_type: BridgeType,
    water: Vec<(i32, i32)>,
    track: TrackType,
}

fn scan_bridge(grid: &Grid, sx: i32, sy: i32, dx: i32, dy: i32, o: Orientation, track: TrackType) -> Option<BridgeOpportunity> {
    let track_kind = if track == TrackType::Road { BuildingType::Road } else { BuildingType::Rail };
    let mut water = Vec::new();
    let (mut x, mut y) = (sx + dx, sy + dy);
    while let Some(t) = grid.get(x, y) {
        match t.building.kind {
            BuildingType::Water => {
                water.push((x, y));
                if water.len() > MAX_BRIDGE_SPAN {
                    return None;
                }
            }
            k if k == track_kind => {
                if water.is_empty() {
                    return None;
                }
                return Some(BridgeOpportunity {
                    orientation: o,
                    bridge_type: bridge_type_for_span(water.len()),
                    water,
                    track,
                });
            }
            _ => return None,
        }
        x += dx;
        y += dy;
    }
    None
}

fn build_bridge(grid: &mut Grid, mut op: BridgeOpportunity) {
    let (fx, fy) = op.water[0];
    let variant = (((fx * 31 + fy * 17) % 100) % bridge_variants(op.bridge_type)) as u8;
    if op.orientation == Orientation::Ns {
        op.water.sort_by_key(|&(x, y)| (x, y));
    } else {
        op.water.sort_by_key(|&(x, y)| (y, x));
    }
    let span = op.water.len();
    for (i, &(x, y)) in op.water.iter().enumerate() {
        let t = grid.at_mut(x, y);
        let mut b = Building::new(BuildingType::Bridge);
        b.level = 0;
        b.powered = true;
        b.watered = true;
        b.bridge_type = Some(op.bridge_type);
        b.bridge_orientation = Some(op.orientation);
        b.bridge_variant = Some(variant);
        b.bridge_position = Some(if i == 0 {
            BridgePosition::Start
        } else if i == span - 1 {
            BridgePosition::End
        } else {
            BridgePosition::Middle
        });
        b.bridge_index = Some(i as u8);
        b.bridge_span = Some(span as u8);
        b.bridge_track_type = Some(op.track);
        t.building = b;
        t.zone = ZoneType::None;
    }
}

/// After a road/rail drag crossing water, spans gaps with bridges.
pub fn create_bridges_on_path(state: &mut GameState, path: &[(i32, i32)], track: TrackType) -> bool {
    if !path.iter().any(|&(x, y)| state.grid.is_water(x, y)) {
        return false;
    }
    let track_kind = if track == TrackType::Road { BuildingType::Road } else { BuildingType::Rail };
    let mut changed = false;
    for &(x, y) in path {
        if state.grid.kind(x, y) != Some(track_kind) {
            continue;
        }
        let dirs = [(-1, 0, Orientation::Ns), (1, 0, Orientation::Ns), (0, -1, Orientation::Ew), (0, 1, Orientation::Ew)];
        for (dx, dy, o) in dirs {
            if let Some(op) = scan_bridge(&state.grid, x, y, dx, dy, o, track) {
                build_bridge(&mut state.grid, op);
                changed = true;
                break;
            }
        }
    }
    if changed {
        state.touch();
    }
    changed
}
