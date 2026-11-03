//! Player edits: placing roads and buildings, bulldozing and bridges.

mod common;

use common::flat_city;
use isocity_core::{session::Session, Building, BuildingType, Command, LocalSession, Tool, TrackType};

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
