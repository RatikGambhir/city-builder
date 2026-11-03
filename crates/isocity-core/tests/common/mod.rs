//! Shared fixtures for the integration tests.

use isocity_core::{new_game, Building, BuildingType, GameState};

/// A 30x30 city with all terrain flattened to grass.
pub fn flat_city() -> GameState {
    let mut s = new_game(30, "Test", Some(1));
    for t in s.grid.tiles_mut() {
        t.building = Building::new(BuildingType::Grass);
    }
    s
}
