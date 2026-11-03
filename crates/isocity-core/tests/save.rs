//! Saving and loading cities.

mod common;

use common::flat_city;
use isocity_core::{new_game, save, sim};

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
