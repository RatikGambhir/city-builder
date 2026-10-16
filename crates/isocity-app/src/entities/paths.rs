//! Breadth-first pathfinding on roads, rails and water.

use isocity_core::{BuildingType as B, Direction, Grid, Orientation};
use std::collections::VecDeque;

/// Can a road vehicle move from `(x, y)` in `dir`? Bridges only allow travel
/// along their orientation.
pub fn can_drive(grid: &Grid, x: i32, y: i32, dir: Direction) -> bool {
    let (dx, dy) = dir.step();
    let (nx, ny) = (x + dx, y + dy);
    if !grid.is_road(nx, ny) {
        return false;
    }
    let along = |t: &isocity_core::Tile| match t.building.bridge_orientation {
        Some(Orientation::Ns) => dir.is_ns(),
        Some(Orientation::Ew) => !dir.is_ns(),
        None => true,
    };
    let here = grid.at(x, y);
    if here.building.kind == B::Bridge && !along(here) {
        return false;
    }
    let next = grid.at(nx, ny);
    if next.building.kind == B::Bridge && !along(next) {
        return false;
    }
    true
}

pub fn road_options(grid: &Grid, x: i32, y: i32) -> Vec<Direction> {
    Direction::ALL
        .iter()
        .copied()
        .filter(|d| can_drive(grid, x, y, *d))
        .collect()
}

fn bfs(
    size: i32,
    start: (i32, i32),
    goal: impl Fn(i32, i32) -> bool,
    passable: impl Fn(i32, i32, i32, i32) -> bool,
    max_nodes: usize,
) -> Option<Vec<(i32, i32)>> {
    if size <= 0 {
        return None;
    }
    let idx = |x: i32, y: i32| (y * size + x) as usize;
    let mut parent = vec![u32::MAX; (size * size) as usize];
    let mut q = VecDeque::new();
    parent[idx(start.0, start.1)] = idx(start.0, start.1) as u32;
    q.push_back(start);
    let mut visited = 0;
    while let Some((x, y)) = q.pop_front() {
        if goal(x, y) {
            let mut path = vec![(x, y)];
            let mut cur = idx(x, y);
            while cur != idx(start.0, start.1) {
                cur = parent[cur] as usize;
                path.push(((cur as i32) % size, (cur as i32) / size));
            }
            path.reverse();
            return Some(path);
        }
        visited += 1;
        if visited > max_nodes {
            return None;
        }
        for (dx, dy) in [(-1, 0), (0, -1), (1, 0), (0, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= size || ny >= size {
                continue;
            }
            if parent[idx(nx, ny)] != u32::MAX || !passable(x, y, nx, ny) {
                continue;
            }
            parent[idx(nx, ny)] = idx(x, y) as u32;
            q.push_back((nx, ny));
        }
    }
    None
}

pub fn road_path(grid: &Grid, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
    if !grid.is_road(from.0, from.1) || !grid.is_road(to.0, to.1) {
        return None;
    }
    bfs(
        grid.size(),
        from,
        |x, y| (x, y) == to,
        |x, y, nx, ny| Direction::from_step(nx - x, ny - y).map_or(false, |d| can_drive(grid, x, y, d)),
        20_000,
    )
}

pub fn rail_path(grid: &Grid, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
    bfs(grid.size(), from, |x, y| (x, y) == to, |_, _, nx, ny| grid.is_rail(nx, ny), 20_000)
}

/// Water path between two water tiles.
pub fn water_path(grid: &Grid, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
    bfs(grid.size(), from, |x, y| (x, y) == to, |_, _, nx, ny| grid.is_water(nx, ny), 20_000)
}

/// A road tile next to the footprint of the building anchored at `(x, y)`.
pub fn road_next_to(grid: &Grid, x: i32, y: i32) -> Option<(i32, i32)> {
    let fp = grid.get(x, y)?.building.kind.size();
    for dy in -1..=fp.height {
        for dx in -1..=fp.width {
            let edge = dx == -1 || dy == -1 || dx == fp.width || dy == fp.height;
            let corner = (dx == -1 || dx == fp.width) && (dy == -1 || dy == fp.height);
            if edge && !corner && grid.is_road(x + dx, y + dy) {
                return Some((x + dx, y + dy));
            }
        }
    }
    None
}

/// Size of the connected water body containing `(x, y)` (capped).
pub fn water_body_size(grid: &Grid, x: i32, y: i32, cap: usize) -> usize {
    if !grid.is_water(x, y) {
        return 0;
    }
    let mut seen = std::collections::HashSet::new();
    let mut q = VecDeque::new();
    q.push_back((x, y));
    seen.insert((x, y));
    while let Some((cx, cy)) = q.pop_front() {
        if seen.len() >= cap {
            break;
        }
        for (dx, dy) in [(-1, 0), (0, -1), (1, 0), (0, 1)] {
            let n = (cx + dx, cy + dy);
            if grid.is_water(n.0, n.1) && seen.insert(n) {
                q.push_back(n);
            }
        }
    }
    seen.len()
}
