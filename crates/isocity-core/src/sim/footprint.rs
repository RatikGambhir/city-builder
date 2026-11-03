//! Road access and multi-tile building footprints: finding, merging and
//! applying the tiles a building occupies.

use crate::buildings::{BuildingType, Footprint};
use crate::types::*;

/// BFS through same-zone tiles looking for a road within `max_distance`.
pub fn has_road_access(grid: &Grid, x: i32, y: i32, max_distance: i32) -> bool {
    let start_zone = grid.at(x, y).zone;
    if start_zone == ZoneType::None {
        return false;
    }
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back((x, y, 0));
    visited.insert((x, y));
    let mut budget = 256;
    while let Some((cx, cy, dist)) = queue.pop_front() {
        if dist >= max_distance {
            continue;
        }
        for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
            if !grid.in_bounds(nx, ny) || !visited.insert((nx, ny)) {
                continue;
            }
            let n = grid.at(nx, ny);
            if matches!(n.building.kind, BuildingType::Road | BuildingType::Bridge) {
                return true;
            }
            if n.zone == start_zone && n.building.kind != BuildingType::Water && budget > 0 {
                budget -= 1;
                queue.push_back((nx, ny, dist + 1));
            }
        }
    }
    false
}

pub(super) fn can_spawn_multi_tile(grid: &Grid, x: i32, y: i32, fp: Footprint, zone: ZoneType) -> bool {
    if x + fp.width > grid.size() || y + fp.height > grid.size() {
        return false;
    }
    for dy in 0..fp.height {
        for dx in 0..fp.width {
            let t = grid.at(x + dx, y + dy);
            if t.zone != zone || !matches!(t.building.kind, BuildingType::Grass | BuildingType::Tree) {
                return false;
            }
        }
    }
    true
}

fn consolidatable(zone: ZoneType, kind: BuildingType) -> bool {
    use BuildingType::*;
    match zone {
        ZoneType::Residential => matches!(kind, HouseSmall | HouseMedium),
        ZoneType::Commercial => matches!(kind, ShopSmall | ShopMedium),
        ZoneType::Industrial => kind == FactorySmall,
        ZoneType::None => false,
    }
}

fn is_mergeable(t: &Tile, zone: ZoneType, exclude: (i32, i32), allow_consolidation: bool) -> bool {
    if (t.x, t.y) == exclude {
        return t.zone == zone
            && !t.building.on_fire
            && !matches!(t.building.kind, BuildingType::Water | BuildingType::Road);
    }
    if t.zone != zone || t.building.on_fire {
        return false;
    }
    match t.building.kind {
        BuildingType::Water | BuildingType::Road | BuildingType::Bridge => false,
        BuildingType::Grass | BuildingType::Tree => true,
        k => allow_consolidation && consolidatable(zone, k),
    }
}

pub(super) fn find_footprint_including_tile(
    grid: &Grid,
    x: i32,
    y: i32,
    fp: Footprint,
    zone: ZoneType,
    allow_consolidation: bool,
) -> Option<(i32, i32)> {
    let size = grid.size();
    let mut best: Option<(i32, i32, f32)> = None;
    for oy in (y - (fp.height - 1))..=y {
        for ox in (x - (fp.width - 1))..=x {
            if ox < 0 || oy < 0 || ox + fp.width > size || oy + fp.height > size {
                continue;
            }
            let ok = (0..fp.height).all(|dy| {
                (0..fp.width).all(|dx| {
                    is_mergeable(grid.at(ox + dx, oy + dy), zone, (x, y), allow_consolidation)
                })
            });
            if !ok {
                continue;
            }
            // Prefer footprints touching roads.
            let mut road_score = 0;
            for dy in 0..fp.height {
                for dx in 0..fp.width {
                    for (ax, ay) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        if grid.is_road_or_bridge(ox + dx + ax, oy + dy + ay) {
                            road_score += 1;
                        }
                    }
                }
            }
            let score = road_score as f32 - (fp.width * fp.height) as f32 * 0.25;
            if best.is_none_or(|b| score > b.2) {
                best = Some((ox, oy, score));
            }
        }
    }
    best.map(|b| (b.0, b.1))
}

/// Writes a building and its `empty` placeholder tiles. Returns nothing; the
/// anchor building lives at `(ox, oy)`.
pub fn apply_building_footprint(
    grid: &mut Grid,
    ox: i32,
    oy: i32,
    kind: BuildingType,
    zone: ZoneType,
    level: u8,
    services: Option<&ServiceCoverage>,
) {
    let fp = kind.size();
    let pollution = kind.stats().pollution;
    for dy in 0..fp.height {
        for dx in 0..fp.width {
            let cell = grid.at_mut(ox + dx, oy + dy);
            if dx == 0 && dy == 0 {
                cell.building = Building::new(kind);
                cell.building.level = level;
                if let Some(s) = services {
                    let i = s.idx(ox, oy);
                    cell.building.powered = s.power[i];
                    cell.building.watered = s.water[i];
                }
                cell.pollution = pollution;
            } else {
                cell.building = Building::new(BuildingType::Empty);
                cell.building.level = 0;
                cell.pollution = 0.0;
            }
            cell.has_rail_overlay = false;
            cell.zone = zone;
        }
    }
}

/// Origin of the multi-tile building covering `(x, y)`, if any.
pub fn find_building_origin(grid: &Grid, x: i32, y: i32) -> Option<(i32, i32, BuildingType)> {
    let tile = grid.get(x, y)?;
    let kind = tile.building.kind;
    if !kind.is_terrain() {
        return if kind.size().is_multi() {
            Some((x, y, kind))
        } else {
            None
        };
    }
    if kind == BuildingType::Empty {
        for dy in 0..4 {
            for dx in 0..4 {
                let (cx, cy) = (x - dx, y - dy);
                if let Some(t) = grid.get(cx, cy) {
                    let k = t.building.kind;
                    if !k.is_terrain() {
                        let fp = k.size();
                        if x >= cx && x < cx + fp.width && y >= cy && y < cy + fp.height {
                            return Some((cx, cy, k));
                        }
                    }
                }
            }
        }
    }
    None
}
