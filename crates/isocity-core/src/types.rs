//! Core data model. Field names serialize in camelCase so cities saved by the
//! IsoCity web app can be imported directly.

use crate::buildings::BuildingType;
use crate::rng::Rng;
use serde::de::Deserializer;
use serde::ser::{SerializeSeq, Serializer};
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

/// Square tile grid stored row-major. Serializes as `Tile[][]` (rows of y).
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    size: i32,
    tiles: Vec<Tile>,
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Demand {
    pub residential: f32,
    pub commercial: f32,
    pub industrial: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub population: i64,
    pub jobs: i64,
    pub money: i64,
    pub income: i64,
    pub expenses: i64,
    pub happiness: f32,
    pub health: f32,
    pub education: f32,
    pub safety: f32,
    pub environment: f32,
    pub demand: Demand,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            population: 0,
            jobs: 0,
            money: 100_000,
            income: 0,
            expenses: 0,
            happiness: 50.0,
            health: 50.0,
            education: 50.0,
            safety: 50.0,
            environment: 75.0,
            demand: Demand {
                residential: 50.0,
                commercial: 30.0,
                industrial: 40.0,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BudgetCategory {
    pub name: String,
    pub funding: f32,
    pub cost: i64,
}

impl BudgetCategory {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            funding: 100.0,
            cost: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKey {
    Police,
    Fire,
    Health,
    Education,
    Transportation,
    Parks,
    Power,
    Water,
}

impl BudgetKey {
    pub const ALL: [BudgetKey; 8] = [
        BudgetKey::Police,
        BudgetKey::Fire,
        BudgetKey::Health,
        BudgetKey::Education,
        BudgetKey::Transportation,
        BudgetKey::Parks,
        BudgetKey::Power,
        BudgetKey::Water,
    ];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub police: BudgetCategory,
    pub fire: BudgetCategory,
    pub health: BudgetCategory,
    pub education: BudgetCategory,
    pub transportation: BudgetCategory,
    pub parks: BudgetCategory,
    pub power: BudgetCategory,
    pub water: BudgetCategory,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            police: BudgetCategory::new("Police"),
            fire: BudgetCategory::new("Fire"),
            health: BudgetCategory::new("Health"),
            education: BudgetCategory::new("Education"),
            transportation: BudgetCategory::new("Transportation"),
            parks: BudgetCategory::new("Parks"),
            power: BudgetCategory::new("Power"),
            water: BudgetCategory::new("Water"),
        }
    }
}

impl Budget {
    pub fn get(&self, key: BudgetKey) -> &BudgetCategory {
        match key {
            BudgetKey::Police => &self.police,
            BudgetKey::Fire => &self.fire,
            BudgetKey::Health => &self.health,
            BudgetKey::Education => &self.education,
            BudgetKey::Transportation => &self.transportation,
            BudgetKey::Parks => &self.parks,
            BudgetKey::Power => &self.power,
            BudgetKey::Water => &self.water,
        }
    }

    pub fn get_mut(&mut self, key: BudgetKey) -> &mut BudgetCategory {
        match key {
            BudgetKey::Police => &mut self.police,
            BudgetKey::Fire => &mut self.fire,
            BudgetKey::Health => &mut self.health,
            BudgetKey::Education => &mut self.education,
            BudgetKey::Transportation => &mut self.transportation,
            BudgetKey::Parks => &mut self.parks,
            BudgetKey::Power => &mut self.power,
            BudgetKey::Water => &mut self.water,
        }
    }

    pub fn total_expenses(&self) -> i64 {
        BudgetKey::ALL
            .iter()
            .map(|k| {
                let c = self.get(*k);
                (c.cost as f32 * c.funding / 100.0).floor() as i64
            })
            .sum()
    }
}

/// Derived per-tile service coverage. Recomputed every tick; never saved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServiceCoverage {
    pub size: i32,
    pub police: Vec<f32>,
    pub fire: Vec<f32>,
    pub health: Vec<f32>,
    pub education: Vec<f32>,
    pub power: Vec<bool>,
    pub water: Vec<bool>,
}

impl ServiceCoverage {
    pub fn new(size: i32) -> Self {
        let n = (size * size).max(0) as usize;
        Self {
            size,
            police: vec![0.0; n],
            fire: vec![0.0; n],
            health: vec![0.0; n],
            education: vec![0.0; n],
            power: vec![false; n],
            water: vec![false; n],
        }
    }

    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.size + x) as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];

    /// Grid step. North is `x - 1`, east is `y - 1` (isometric convention).
    pub fn step(self) -> (i32, i32) {
        match self {
            Direction::North => (-1, 0),
            Direction::East => (0, -1),
            Direction::South => (1, 0),
            Direction::West => (0, 1),
        }
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }

    pub fn from_step(dx: i32, dy: i32) -> Option<Direction> {
        match (dx, dy) {
            (-1, 0) => Some(Direction::North),
            (0, -1) => Some(Direction::East),
            (1, 0) => Some(Direction::South),
            (0, 1) => Some(Direction::West),
            _ => None,
        }
    }

    pub fn is_ns(self) -> bool {
        matches!(self, Direction::North | Direction::South)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdjacentCity {
    pub id: String,
    pub name: String,
    pub direction: Direction,
    pub connected: bool,
    pub discovered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaterBodyKind {
    Lake,
    Ocean,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaterBody {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: WaterBodyKind,
    pub tiles: Vec<Point>,
    pub center_x: f32,
    pub center_y: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon: String,
    #[serde(default)]
    pub timestamp: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdvisorMessage {
    pub name: String,
    pub icon: String,
    pub messages: Vec<String>,
    pub priority: Priority,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub year: i32,
    pub month: i32,
    pub population: i64,
    pub money: i64,
    pub happiness: f32,
}

fn default_true() -> bool {
    true
}
fn default_year() -> i32 {
    2024
}
fn one() -> i32 {
    1
}
fn default_tax() -> f32 {
    9.0
}
fn default_speed() -> u8 {
    1
}

/// Complete, serializable state of one city.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    #[serde(default)]
    pub id: String,
    pub grid: Grid,
    pub grid_size: i32,
    pub city_name: String,
    #[serde(default = "default_year")]
    pub year: i32,
    #[serde(default = "one")]
    pub month: i32,
    #[serde(default = "one")]
    pub day: i32,
    #[serde(default)]
    pub hour: i32,
    #[serde(default)]
    pub tick: i32,
    #[serde(default = "default_speed")]
    pub speed: u8,
    #[serde(default = "default_tax")]
    pub tax_rate: f32,
    #[serde(default)]
    pub effective_tax_rate: Option<f32>,
    #[serde(default)]
    pub stats: Stats,
    #[serde(default)]
    pub budget: Budget,
    #[serde(default)]
    pub notifications: Vec<Notification>,
    #[serde(default)]
    pub advisor_messages: Vec<AdvisorMessage>,
    #[serde(default)]
    pub history: Vec<HistoryPoint>,
    #[serde(default = "default_true")]
    pub disasters_enabled: bool,
    #[serde(default)]
    pub adjacent_cities: Vec<AdjacentCity>,
    #[serde(default)]
    pub water_bodies: Vec<WaterBody>,
    /// Bumped whenever the grid changes; renderers use it to invalidate caches.
    #[serde(default)]
    pub game_version: u64,
    /// Deterministic random source for the simulation.
    #[serde(default)]
    pub rng: Rng,
    #[serde(skip)]
    pub services: ServiceCoverage,
}

impl GameState {
    pub fn effective_tax_rate(&self) -> f32 {
        self.effective_tax_rate.unwrap_or(self.tax_rate)
    }

    pub fn net_income(&self) -> i64 {
        self.stats.income - self.stats.expenses
    }

    pub fn tile(&self, x: i32, y: i32) -> Option<&Tile> {
        self.grid.get(x, y)
    }

    pub fn push_notification(&mut self, title: &str, description: &str, icon: &str) {
        let id = self.rng.uuid();
        self.notifications.insert(
            0,
            Notification {
                id,
                title: title.to_string(),
                description: description.to_string(),
                icon: icon.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as f64)
                    .unwrap_or(0.0),
            },
        );
        self.notifications.truncate(10);
    }

    /// Mark the grid as changed for render caches.
    pub fn touch(&mut self) {
        self.game_version = self.game_version.wrapping_add(1);
    }
}
