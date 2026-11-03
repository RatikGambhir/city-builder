//! Tiles and the buildings on them, plus zone, bridge and track enums.

use crate::buildings::BuildingType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoneType {
    #[default]
    None,
    Residential,
    Commercial,
    Industrial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeType {
    Small,
    Medium,
    Large,
    Suspension,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Ns,
    Ew,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgePosition {
    Start,
    Middle,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrackType {
    #[default]
    Road,
    Rail,
}

fn hundred() -> f32 {
    100.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Building {
    #[serde(rename = "type")]
    pub kind: BuildingType,
    #[serde(default)]
    pub level: u8,
    #[serde(default)]
    pub population: u32,
    #[serde(default)]
    pub jobs: u32,
    #[serde(default)]
    pub powered: bool,
    #[serde(default)]
    pub watered: bool,
    #[serde(default)]
    pub on_fire: bool,
    #[serde(default)]
    pub fire_progress: f32,
    #[serde(default)]
    pub age: f32,
    #[serde(default = "hundred")]
    pub construction_progress: f32,
    #[serde(default)]
    pub abandoned: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub flipped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_type: Option<BridgeType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_orientation: Option<Orientation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_variant: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_position: Option<BridgePosition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_index: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_span: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge_track_type: Option<TrackType>,
}

impl Building {
    pub fn new(kind: BuildingType) -> Self {
        Self {
            kind,
            level: if matches!(
                kind,
                BuildingType::Grass | BuildingType::Empty | BuildingType::Water
            ) {
                0
            } else {
                1
            },
            population: 0,
            jobs: 0,
            powered: false,
            watered: false,
            on_fire: false,
            fire_progress: 0.0,
            age: 0.0,
            construction_progress: if kind.needs_construction() && kind != BuildingType::Rail {
                0.0
            } else {
                100.0
            },
            abandoned: false,
            flipped: false,
            bridge_type: None,
            bridge_orientation: None,
            bridge_variant: None,
            bridge_position: None,
            bridge_index: None,
            bridge_span: None,
            bridge_track_type: None,
        }
    }

    pub fn under_construction(&self) -> bool {
        self.construction_progress < 100.0
    }

    pub fn is_rail_bridge(&self) -> bool {
        self.kind == BuildingType::Bridge && self.bridge_track_type == Some(TrackType::Rail)
    }
}

impl Default for Building {
    fn default() -> Self {
        Building::new(BuildingType::Grass)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub zone: ZoneType,
    pub building: Building,
    #[serde(default = "fifty")]
    pub land_value: f32,
    #[serde(default)]
    pub pollution: f32,
    #[serde(default)]
    pub crime: f32,
    #[serde(default)]
    pub traffic: f32,
    #[serde(default)]
    pub has_subway: bool,
    #[serde(default)]
    pub has_rail_overlay: bool,
}

fn fifty() -> f32 {
    50.0
}

impl Tile {
    pub fn new(x: i32, y: i32, kind: BuildingType) -> Self {
        Self {
            x,
            y,
            zone: ZoneType::None,
            building: Building::new(kind),
            land_value: 50.0,
            pollution: 0.0,
            crime: 0.0,
            traffic: 0.0,
            has_subway: false,
            has_rail_overlay: false,
        }
    }

    pub fn kind(&self) -> BuildingType {
        self.building.kind
    }
}
