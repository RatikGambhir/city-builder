//! Isometric grid <-> world coordinate conversion.

use isocity_app::iso::{tile_center, world_to_grid};

#[test]
fn roundtrip() {
    for (x, y) in [(0, 0), (5, 9), (40, 3)] {
        let (cx, cy) = tile_center(x as f32, y as f32);
        assert_eq!(world_to_grid(cx, cy), (x, y));
    }
}
