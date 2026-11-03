//! A flat per-tile summary of the grid (roads, rail, water, bridges) that the
//! painters and entities query.

use isocity_core::{
    sim::find_building_origin, BridgePosition, BridgeType, BuildingType as B, GameState, Orientation, TrackType,
    ZoneType,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BridgeInfo {
    pub kind: BridgeType,
    pub orient: Orientation,
    pub variant: u8,
    pub pos: BridgePosition,
    pub index: u8,
    pub span: u8,
    pub rail: bool,
}

/// Everything about a tile that affects the static ground layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileInfo {
    pub in_bounds: bool,
    pub kind: B,
    pub zone: ZoneType,
    pub rail_overlay: bool,
    pub park: bool,
    pub grey_base: bool,
    pub foundation: bool,
    pub station: bool,
    pub marina_pier: bool,
    pub bridge: Option<BridgeInfo>,
}

impl Default for TileInfo {
    fn default() -> Self {
        Self {
            in_bounds: false,
            kind: B::Grass,
            zone: ZoneType::None,
            rail_overlay: false,
            park: false,
            grey_base: false,
            foundation: false,
            station: false,
            marina_pier: false,
            bridge: None,
        }
    }
}

/// Precomputed [`TileInfo`] for a whole grid.
#[derive(Clone)]
pub struct TileMap {
    pub size: i32,
    pub tiles: Arc<Vec<TileInfo>>,
}

impl TileMap {
    pub fn build(state: &GameState) -> Self {
        let grid = &state.grid;
        let size = grid.size();
        let mut tiles = vec![TileInfo::default(); (size * size) as usize];
        for t in grid.iter() {
            let b = &t.building;
            let (park, part_of_building, origin_kind, origin_foundation) = if b.kind == B::Empty {
                match find_building_origin(grid, t.x, t.y) {
                    Some((ox, oy, k)) => (
                        k.is_park(),
                        true,
                        Some(k),
                        grid.at(ox, oy).building.construction_progress < 40.0,
                    ),
                    None => (false, false, None, false),
                }
            } else {
                (b.kind.is_park(), false, Some(b.kind), b.construction_progress < 40.0 && b.kind.needs_construction())
            };
            let direct = !park && !matches!(b.kind, B::Grass | B::Empty | B::Water | B::Road | B::Bridge | B::Rail | B::Tree);
            let grey_base = (direct || (b.kind == B::Empty && part_of_building)) && !park;
            let is_building = origin_kind.is_some_and(|k| !k.is_terrain());
            tiles[(t.y * size + t.x) as usize] = TileInfo {
                in_bounds: true,
                kind: b.kind,
                zone: t.zone,
                rail_overlay: t.has_rail_overlay,
                park,
                grey_base,
                foundation: is_building && origin_foundation,
                station: origin_kind == Some(B::RailStation),
                marina_pier: matches!(origin_kind, Some(B::MarinaDocksSmall) | Some(B::PierLarge)),
                bridge: if b.kind == B::Bridge {
                    Some(BridgeInfo {
                        kind: b.bridge_type.unwrap_or(BridgeType::Large),
                        orient: b.bridge_orientation.unwrap_or(Orientation::Ns),
                        variant: b.bridge_variant.unwrap_or(0),
                        pos: b.bridge_position.unwrap_or(BridgePosition::Middle),
                        index: b.bridge_index.unwrap_or(0),
                        span: b.bridge_span.unwrap_or(1),
                        rail: b.bridge_track_type == Some(TrackType::Rail),
                    })
                } else {
                    None
                },
            };
        }
        Self {
            size,
            tiles: Arc::new(tiles),
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> TileInfo {
        if x < 0 || y < 0 || x >= self.size || y >= self.size {
            TileInfo::default()
        } else {
            self.tiles[(y * self.size + x) as usize]
        }
    }

    pub fn is_water(&self, x: i32, y: i32) -> bool {
        let t = self.get(x, y);
        t.in_bounds && t.kind == B::Water
    }

    pub fn has_road(&self, x: i32, y: i32) -> bool {
        let t = self.get(x, y);
        match t.kind {
            B::Road => t.in_bounds,
            B::Bridge => t.bridge.is_none_or(|b| !b.rail),
            _ => false,
        }
    }

    pub fn has_rail(&self, x: i32, y: i32) -> bool {
        let t = self.get(x, y);
        t.in_bounds
            && (t.kind == B::Rail
                || t.station
                || (t.kind == B::Road && t.rail_overlay)
                || t.bridge.is_some_and(|b| b.rail))
    }

    pub fn road_count(&self, x: i32, y: i32) -> u8 {
        self.has_road(x - 1, y) as u8 + self.has_road(x, y - 1) as u8 + self.has_road(x + 1, y) as u8 + self.has_road(x, y + 1) as u8
    }

    pub fn is_intersection(&self, x: i32, y: i32) -> bool {
        self.has_road(x, y) && self.road_count(x, y) >= 3
    }

    /// Hash of all tiles in a rectangle (used to detect chunk changes).
    pub fn region_hash(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for y in y0..y1 {
            for x in x0..x1 {
                self.get(x, y).hash(&mut h);
            }
        }
        h.finish()
    }
}
