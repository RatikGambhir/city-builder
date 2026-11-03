//! The square tile grid and its compact (de)serialization.

use crate::buildings::BuildingType;
use serde::de::Deserializer;
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use super::tile::{Tile, TrackType};

/// Square tile grid stored row-major. Serializes as `Tile[][]` (rows of y).
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub(super) size: i32,
    pub(super) tiles: Vec<Tile>,
}

impl Grid {
    pub fn new(size: i32, mut f: impl FnMut(i32, i32) -> Tile) -> Self {
        let mut tiles = Vec::with_capacity((size * size) as usize);
        for y in 0..size {
            for x in 0..size {
                tiles.push(f(x, y));
            }
        }
        Self { size, tiles }
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.size && y < self.size
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<&Tile> {
        if self.in_bounds(x, y) {
            Some(&self.tiles[(y * self.size + x) as usize])
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut Tile> {
        if self.in_bounds(x, y) {
            Some(&mut self.tiles[(y * self.size + x) as usize])
        } else {
            None
        }
    }

    /// Panicking accessor for coordinates known to be in bounds.
    #[inline]
    pub fn at(&self, x: i32, y: i32) -> &Tile {
        &self.tiles[(y * self.size + x) as usize]
    }

    #[inline]
    pub fn at_mut(&mut self, x: i32, y: i32) -> &mut Tile {
        &mut self.tiles[(y * self.size + x) as usize]
    }

    #[inline]
    pub fn kind(&self, x: i32, y: i32) -> Option<BuildingType> {
        self.get(x, y).map(|t| t.building.kind)
    }

    pub fn is(&self, x: i32, y: i32, kind: BuildingType) -> bool {
        self.kind(x, y) == Some(kind)
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    pub fn tiles_mut(&mut self) -> &mut [Tile] {
        &mut self.tiles
    }

    pub fn iter(&self) -> impl Iterator<Item = &Tile> {
        self.tiles.iter()
    }

    /// Road or road-carrying bridge (what cars can drive on).
    pub fn is_road(&self, x: i32, y: i32) -> bool {
        match self.get(x, y) {
            Some(t) => match t.building.kind {
                BuildingType::Road => true,
                BuildingType::Bridge => t.building.bridge_track_type != Some(TrackType::Rail),
                _ => false,
            },
            None => false,
        }
    }

    pub fn is_road_or_bridge(&self, x: i32, y: i32) -> bool {
        matches!(
            self.kind(x, y),
            Some(BuildingType::Road) | Some(BuildingType::Bridge)
        )
    }

    pub fn is_water(&self, x: i32, y: i32) -> bool {
        self.is(x, y, BuildingType::Water)
    }

    /// Rail track, road with rail overlay, or rail bridge.
    pub fn is_rail(&self, x: i32, y: i32) -> bool {
        match self.get(x, y) {
            Some(t) => {
                t.building.kind == BuildingType::Rail
                    || (t.building.kind == BuildingType::Road && t.has_rail_overlay)
                    || t.building.is_rail_bridge()
            }
            None => false,
        }
    }
}

impl Serialize for Grid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.size as usize))?;
        for row in self.tiles.chunks(self.size.max(1) as usize) {
            seq.serialize_element(row)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for Grid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows: Vec<Vec<Tile>> = Vec::deserialize(deserializer)?;
        let size = rows.len() as i32;
        let mut tiles = Vec::with_capacity((size * size) as usize);
        for (y, row) in rows.into_iter().enumerate() {
            if row.len() as i32 != size {
                return Err(serde::de::Error::custom("grid must be square"));
            }
            for (x, mut t) in row.into_iter().enumerate() {
                t.x = x as i32;
                t.y = y as i32;
                tiles.push(t);
            }
        }
        Ok(Grid { size, tiles })
    }
}
