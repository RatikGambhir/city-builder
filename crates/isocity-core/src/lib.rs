//! # isocity-core
//!
//! The simulation engine behind the IsoCity desktop app: an isometric
//! tile-based city with zoning (residential, commercial, industrial), service
//! coverage, utilities, demand-driven growth, abandonment, pollution, fires and
//! an economy. It has no UI dependencies.
//!
//! All mutations flow through [`command::Command`] values applied by a
//! [`session::Session`], which keeps the door open for online play.

pub mod buildings;
pub mod command;
pub mod names;
pub mod place;
pub mod rng;
pub mod save;
pub mod session;
pub mod sim;
pub mod terrain;
pub mod tools;
pub mod types;

pub use buildings::{BuildingStats, BuildingType, Footprint};
pub use command::{Command, CommandResult, Rejection};
pub use rng::Rng;
pub use session::{LocalSession, Session};
pub use tools::Tool;
pub use types::*;

pub const DEFAULT_GRID_SIZE: i32 = 70;

fn adjacent_cities(rng: &mut Rng) -> Vec<AdjacentCity> {
    let mut used: Vec<String> = Vec::new();
    Direction::ALL
        .iter()
        .map(|&direction| {
            let mut name = names::city_name(rng);
            for _ in 0..20 {
                if !used.contains(&name) {
                    break;
                }
                name = names::city_name(rng);
            }
            used.push(name.clone());
            let id = match direction {
                Direction::North => "city-north",
                Direction::East => "city-east",
                Direction::South => "city-south",
                Direction::West => "city-west",
            };
            AdjacentCity {
                id: id.to_string(),
                name,
                direction,
                connected: false,
                discovered: false,
            }
        })
        .collect()
}

/// Creates a fresh city on generated terrain.
pub fn new_game(size: i32, city_name: &str, seed: Option<u64>) -> GameState {
    let mut rng = seed.map(Rng::new).unwrap_or_else(Rng::from_entropy);
    let (grid, water_bodies) = terrain::generate(size, &mut rng);
    let services = sim::calculate_service_coverage(&grid);
    GameState {
        id: rng.uuid(),
        grid,
        grid_size: size,
        city_name: city_name.to_string(),
        year: 2024,
        month: 1,
        day: 1,
        hour: 12,
        tick: 0,
        speed: 1,
        tax_rate: 9.0,
        effective_tax_rate: Some(9.0),
        stats: Stats::default(),
        budget: Budget::default(),
        notifications: Vec::new(),
        advisor_messages: Vec::new(),
        history: Vec::new(),
        disasters_enabled: true,
        adjacent_cities: adjacent_cities(&mut rng),
        water_bodies,
        game_version: 0,
        services,
        rng,
    }
}

/// Fixes up a deserialized state (older saves, web imports) so it can run.
pub fn normalize_loaded(mut state: GameState) -> GameState {
    state.grid_size = state.grid.size();
    if state.id.is_empty() {
        state.id = state.rng.uuid();
    }
    if state.adjacent_cities.is_empty() {
        state.adjacent_cities = adjacent_cities(&mut state.rng);
    }
    if state.effective_tax_rate.is_none() {
        state.effective_tax_rate = Some(state.tax_rate);
    }
    state.month = state.month.clamp(1, 12);
    state.day = state.day.clamp(1, 30);
    state.services = sim::calculate_service_coverage(&state.grid);
    sim::update_budget_costs(&state.grid, &mut state.budget);
    state.touch();
    state
}
