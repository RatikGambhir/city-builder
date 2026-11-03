//! Core data model. Field names serialize in camelCase so cities saved by the
//! IsoCity web app can be imported directly.

mod tile;
mod grid;
mod economy;
mod coverage;
mod geography;
mod messages;
mod state;

pub use tile::{BridgePosition, BridgeType, Building, Orientation, Tile, TrackType, ZoneType};
pub use grid::Grid;
pub use economy::{Budget, BudgetCategory, BudgetKey, Demand, Stats};
pub use coverage::ServiceCoverage;
pub use geography::{AdjacentCity, Direction, Point, WaterBody, WaterBodyKind};
pub use messages::{AdvisorMessage, HistoryPoint, Notification, Priority};
pub use state::GameState;

