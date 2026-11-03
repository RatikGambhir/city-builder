//! Running the simulation: zone growth and determinism.

mod common;

use common::flat_city;
use isocity_core::{new_game, session::Session, Command, LocalSession, Tool, ZoneType};

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
