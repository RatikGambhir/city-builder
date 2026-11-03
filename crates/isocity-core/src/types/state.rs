//! The complete, serializable game state.

use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use super::coverage::ServiceCoverage;
use super::economy::{Budget, Stats};
use super::geography::{AdjacentCity, WaterBody};
use super::grid::Grid;
use super::messages::{AdvisorMessage, HistoryPoint, Notification};
use super::tile::Tile;

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
