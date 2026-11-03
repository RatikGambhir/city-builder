//! Cities exported by the IsoCity web app load and simulate.

use isocity_core::{save, session::Session, LocalSession};

#[test]
fn bundled_examples_load_and_tick() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/examples");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let bytes = std::fs::read(&path).unwrap();
        let state = save::decode(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(state.grid.size() > 0);
        let pop = state.stats.population;
        let mut s = LocalSession::new(state);
        for _ in 0..5 {
            s.step();
        }
        assert!(s.state().stats.population > 0 || pop == 0);
        count += 1;
    }
    assert!(count >= 1);
}
