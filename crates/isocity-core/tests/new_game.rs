//! Generating a new city.

use isocity_core::{new_game, BuildingType};

#[test]
fn new_game_has_water_and_trees() {
    let s = new_game(70, "Test", Some(3));
    assert_eq!(s.grid.size(), 70);
    assert!(s.grid.iter().any(|t| t.kind() == BuildingType::Water));
    assert!(s.grid.iter().any(|t| t.kind() == BuildingType::Tree));
    assert_eq!(s.adjacent_cities.len(), 4);
}
