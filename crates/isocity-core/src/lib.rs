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

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_city() -> GameState {
        let mut s = new_game(30, "Test", Some(1));
        for t in s.grid.tiles_mut() {
            t.building = Building::new(BuildingType::Grass);
        }
        s
    }

    #[test]
    fn new_game_has_water_and_trees() {
        let s = new_game(70, "Test", Some(3));
        assert_eq!(s.grid.size(), 70);
        assert!(s.grid.iter().any(|t| t.kind() == BuildingType::Water));
        assert!(s.grid.iter().any(|t| t.kind() == BuildingType::Tree));
        assert_eq!(s.adjacent_cities.len(), 4);
    }

    #[test]
    fn placing_road_costs_money() {
        let mut s = LocalSession::new(flat_city());
        let before = s.state().stats.money;
        s.submit(Command::ApplyTool { tool: Tool::Road, x: 5, y: 5 }).unwrap();
        assert_eq!(s.state().grid.kind(5, 5), Some(BuildingType::Road));
        assert_eq!(s.state().stats.money, before - 25);
        // Second placement is a no-op and free.
        assert!(s.submit(Command::ApplyTool { tool: Tool::Road, x: 5, y: 5 }).is_err());
        assert_eq!(s.state().stats.money, before - 25);
    }

    #[test]
    fn multi_tile_building_and_bulldoze() {
        let mut s = LocalSession::new(flat_city());
        s.submit(Command::ApplyTool { tool: Tool::PowerPlant, x: 10, y: 10 }).unwrap();
        assert_eq!(s.state().grid.kind(10, 10), Some(BuildingType::PowerPlant));
        assert_eq!(s.state().grid.kind(11, 11), Some(BuildingType::Empty));
        s.submit(Command::ApplyTool { tool: Tool::Bulldoze, x: 11, y: 11 }).unwrap();
        assert_eq!(s.state().grid.kind(10, 10), Some(BuildingType::Grass));
        assert_eq!(s.state().grid.kind(11, 10), Some(BuildingType::Grass));
    }

    #[test]
    fn zones_grow_into_buildings() {
        let mut s = LocalSession::new(flat_city());
        s.submit(Command::ApplyTool { tool: Tool::PowerPlant, x: 22, y: 5 }).unwrap();
        s.submit(Command::ApplyTool { tool: Tool::WaterTower, x: 22, y: 14 }).unwrap();
        for x in 2..20 {
            s.submit(Command::ApplyTool { tool: Tool::Road, x, y: 10 }).unwrap();
            s.submit(Command::ApplyTool { tool: Tool::ZoneResidential, x, y: 11 }).unwrap();
            s.submit(Command::ApplyTool { tool: Tool::ZoneCommercial, x, y: 9 }).unwrap();
        }
        for _ in 0..300 {
            s.step();
        }
        let built = s
            .state()
            .grid
            .iter()
            .filter(|t| t.zone != ZoneType::None && !t.kind().is_terrain())
            .count();
        assert!(built > 5, "expected growth, got {built}");
        assert!(s.state().stats.population > 0);
    }

    #[test]
    fn bridge_spans_water() {
        let mut st = flat_city();
        for x in 0..30 {
            for y in 12..15 {
                st.grid.at_mut(x, y).building = Building::new(BuildingType::Water);
            }
        }
        let mut s = LocalSession::new(st);
        let path: Vec<(i32, i32)> = (8..18).map(|y| (5, y)).collect();
        for &(x, y) in &path {
            let _ = s.submit(Command::ApplyTool { tool: Tool::Road, x, y });
        }
        s.submit(Command::FinishTrackDrag { path, track: TrackType::Road }).unwrap();
        for y in 12..15 {
            assert_eq!(s.state().grid.kind(5, y), Some(BuildingType::Bridge));
        }
        assert!(s.state().grid.is_road(5, 13));
    }

    #[test]
    fn save_roundtrip() {
        let mut s = flat_city();
        sim::simulate_tick(&mut s);
        let bytes = save::encode(&s).unwrap();
        let back = save::decode(&bytes).unwrap();
        assert_eq!(back.grid, s.grid);
        assert_eq!(back.city_name, "Test");
    }

    #[test]
    fn save_manager_lists_multiple_cities() {
        let dir = std::env::temp_dir().join(format!("isocity-test-{}", save::now_ms()));
        let mgr = save::SaveManager::new(&dir);
        let a = new_game(20, "Alpha", Some(1));
        let b = new_game(20, "Beta", Some(2));
        mgr.save(&a).unwrap();
        mgr.save(&b).unwrap();
        let list = mgr.list();
        assert_eq!(list.len(), 2);
        assert_eq!(mgr.load(&a.id).unwrap().city_name, "Alpha");
        mgr.delete(&a.id).unwrap();
        assert_eq!(mgr.list().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn deterministic_simulation() {
        let mut a = LocalSession::new(new_game(40, "A", Some(9)));
        let mut b = LocalSession::new(new_game(40, "A", Some(9)));
        for s in [&mut a, &mut b] {
            for x in 5..30 {
                let _ = s.submit(Command::ApplyTool { tool: Tool::Road, x, y: 20 });
                let _ = s.submit(Command::ApplyTool { tool: Tool::ZoneResidential, x, y: 21 });
            }
            for _ in 0..100 {
                s.step();
            }
        }
        assert_eq!(a.state().grid, b.state().grid);
        assert_eq!(a.state().stats.population, b.state().stats.population);
    }
}
