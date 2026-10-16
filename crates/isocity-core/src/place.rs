//! Player edits to the grid: placing buildings and zones, bulldozing,
//! bridges, subways, terraforming and growing/shrinking the map.

use crate::buildings::BuildingType;
use crate::sim::{apply_building_footprint, calculate_service_coverage, find_building_origin};
use crate::terrain;
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

fn clear_tile(t: &mut Tile, kind: BuildingType) {
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

pub fn place_subway(state: &mut GameState, x: i32, y: i32) -> bool {
    match state.grid.get_mut(x, y) {
        Some(t) if t.building.kind != BuildingType::Water && !t.has_subway => {
            t.has_subway = true;
            true
        }
        _ => false,
    }
}

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
