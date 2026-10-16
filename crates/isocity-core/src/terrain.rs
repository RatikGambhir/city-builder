//! Procedural terrain: grass, round lakes, organic ocean coastlines and trees.

use crate::buildings::BuildingType;
use crate::names;
use crate::rng::Rng;
use crate::types::{Building, Grid, Point, Tile, WaterBody, WaterBodyKind};

fn noise2d(x: f64, y: f64, seed: f64) -> f64 {
    let n = (x * 12.9898 + y * 78.233 + seed).sin() * 43758.5453123;
    n - n.floor()
}

fn smooth_noise(x: f64, y: f64, seed: f64) -> f64 {
    let corners = (noise2d(x - 1.0, y - 1.0, seed)
        + noise2d(x + 1.0, y - 1.0, seed)
        + noise2d(x - 1.0, y + 1.0, seed)
        + noise2d(x + 1.0, y + 1.0, seed))
        / 16.0;
    let sides = (noise2d(x - 1.0, y, seed)
        + noise2d(x + 1.0, y, seed)
        + noise2d(x, y - 1.0, seed)
        + noise2d(x, y + 1.0, seed))
        / 8.0;
    corners + sides + noise2d(x, y, seed) / 4.0
}

fn interpolated_noise(x: f64, y: f64, seed: f64) -> f64 {
    let ix = x.floor();
    let fx = x - ix;
    let iy = y.floor();
    let fy = y - iy;
    let v1 = smooth_noise(ix, iy, seed);
    let v2 = smooth_noise(ix + 1.0, iy, seed);
    let v3 = smooth_noise(ix, iy + 1.0, seed);
    let v4 = smooth_noise(ix + 1.0, iy + 1.0, seed);
    let i1 = v1 * (1.0 - fx) + v2 * fx;
    let i2 = v3 * (1.0 - fx) + v4 * fx;
    i1 * (1.0 - fy) + i2 * fy
}

/// Fractal value noise in roughly `[0, 1]`.
pub fn perlin(x: f64, y: f64, seed: f64, octaves: u32) -> f64 {
    let mut total = 0.0;
    let mut frequency = 0.05;
    let mut amplitude = 1.0;
    let mut max = 0.0;
    for i in 0..octaves {
        total += interpolated_noise(x * frequency, y * frequency, seed + i as f64 * 100.0) * amplitude;
        max += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    total / max
}

fn make_water(grid: &mut Grid, x: i32, y: i32) {
    let t = grid.at_mut(x, y);
    t.building = Building::new(BuildingType::Water);
    t.land_value = 60.0;
}

fn generate_lakes(grid: &mut Grid, rng: &mut Rng, seed: f64) -> Vec<WaterBody> {
    let size = grid.size();
    let lake_noise = |x: i32, y: i32| perlin(x as f64, y as f64, seed + 1000.0, 3);
    let min_edge = 8.max((size as f64 * 0.15) as i32);
    let min_between = (size as f64 * 0.2).max(10.0);

    let mut centers: Vec<(i32, i32, f64)> = Vec::new();
    let mut threshold = 0.5;
    for _ in 0..3 {
        centers.clear();
        for y in min_edge..(size - min_edge) {
            for x in min_edge..(size - min_edge) {
                let n = lake_noise(x, y);
                if n < threshold
                    && !centers.iter().any(|c| {
                        (((x - c.0).pow(2) + (y - c.1).pow(2)) as f64).sqrt() < min_between
                    })
                {
                    centers.push((x, y, n));
                }
            }
        }
        if centers.len() >= 2 {
            break;
        }
        threshold += 0.1;
    }

    let safe = min_edge + 5;
    let quarter = safe.max(size / 4);
    let three_quarter = (size - safe).min(size * 3 / 4);
    if centers.is_empty() {
        centers.push((quarter, quarter, 0.0));
        centers.push((three_quarter, three_quarter, 0.0));
    } else if centers.len() == 1 {
        let e = centers[0];
        let nx = if e.0 > size / 2 { quarter } else { three_quarter };
        let ny = if e.1 > size / 2 { quarter } else { three_quarter };
        centers.push((nx, ny, 0.0));
    }

    centers.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    let num_lakes = 2 + rng.below(2);
    centers.truncate(num_lakes);

    let in_range = |x: i32, y: i32| x >= min_edge && x < size - min_edge && y >= min_edge && y < size - min_edge;
    const DIRS: [(i32, i32); 8] = [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (-1, 1), (1, -1), (1, 1)];

    let mut bodies = Vec::new();
    let mut used_names = Vec::<String>::new();
    for (cx, cy, _) in centers {
        if !grid.in_bounds(cx, cy) {
            continue;
        }
        let target = 40 + rng.below(41);
        let mut lake: Vec<Point> = vec![Point { x: cx, y: cy }];
        let mut in_lake = std::collections::HashSet::new();
        in_lake.insert((cx, cy));
        let mut candidates: Vec<(i32, i32, f64, f64)> = Vec::new();
        let mut in_candidates = std::collections::HashSet::new();
        for (dx, dy) in DIRS {
            let (nx, ny) = (cx + dx, cy + dy);
            if in_range(nx, ny) {
                candidates.push((nx, ny, ((dx * dx + dy * dy) as f64).sqrt(), lake_noise(nx, ny)));
                in_candidates.insert((nx, ny));
            }
        }
        while lake.len() < target && !candidates.is_empty() {
            // Closer tiles first (rounder lakes); within a distance band prefer
            // lower noise (organic edges).
            candidates.sort_by(|a, b| {
                let (ba, bb) = ((a.2 * 2.0).floor(), (b.2 * 2.0).floor());
                ba.total_cmp(&bb).then(a.3.total_cmp(&b.3))
            });
            let pick = rng.below(5.min(candidates.len()));
            let (px, py, _, _) = candidates.remove(pick);
            in_candidates.remove(&(px, py));
            if in_lake.contains(&(px, py)) || grid.is_water(px, py) {
                continue;
            }
            lake.push(Point { x: px, y: py });
            in_lake.insert((px, py));
            for (dx, dy) in DIRS {
                let (nx, ny) = (px + dx, py + dy);
                if in_range(nx, ny) && !in_lake.contains(&(nx, ny)) && !in_candidates.contains(&(nx, ny)) {
                    let d = ((((nx - cx).pow(2) + (ny - cy).pow(2)) as f64)).sqrt();
                    candidates.push((nx, ny, d, lake_noise(nx, ny)));
                    in_candidates.insert((nx, ny));
                }
            }
        }
        for p in &lake {
            make_water(grid, p.x, p.y);
        }
        let ax = lake.iter().map(|p| p.x as f32).sum::<f32>() / lake.len() as f32;
        let ay = lake.iter().map(|p| p.y as f32).sum::<f32>() / lake.len() as f32;
        let mut name = names::lake_name(rng);
        for _ in 0..20 {
            if !used_names.contains(&name) {
                break;
            }
            name = names::lake_name(rng);
        }
        used_names.push(name.clone());
        bodies.push(WaterBody {
            id: format!("lake-{}", bodies.len()),
            name,
            kind: WaterBodyKind::Lake,
            tiles: lake,
            center_x: ax.round(),
            center_y: ay.round(),
        });
    }
    bodies
}

fn generate_oceans(grid: &mut Grid, rng: &mut Rng, seed: f64) -> Vec<WaterBody> {
    let size = grid.size();
    let coast = |x: f64, y: f64| perlin(x, y, seed + 2000.0, 3);
    let base_depth = 4.max((size as f64 * 0.12) as i32) as f64;
    let depth_var = 4.max((size as f64 * 0.08) as i32) as f64;
    let max_depth = (size as f64 * 0.18) as i32;

    let mut edges: Vec<(&str, Vec<Point>)> = Vec::new();
    let gen_edge = |grid: &mut Grid, rng: &mut Rng, horizontal: bool, edge_pos: i32, inward: i32| {
        let mut tiles = Vec::new();
        let span_start = (size as f64 * (0.05 + rng.next_f64() * 0.25)) as i32;
        let span_end = (size as f64 * (0.7 + rng.next_f64() * 0.25)) as i32;
        for i in span_start..span_end {
            let fade = ((i - span_start) as f64 / 5.0)
                .min((span_end - i) as f64 / 5.0)
                .min(1.0);
            let (ep, fi) = (edge_pos as f64, i as f64);
            let coarse = if horizontal {
                coast(fi * 0.08, ep * 0.08)
            } else {
                coast(ep * 0.08, fi * 0.08)
            };
            let fine = if horizontal {
                coast(fi * 0.25, ep * 0.25 + 500.0)
            } else {
                coast(ep * 0.25 + 500.0, fi * 0.25)
            };
            let n = coarse * 0.6 + fine * 0.4;
            let raw = base_depth + (n - 0.5) * depth_var * 2.5;
            let local = ((raw * fade).floor() as i32).min(max_depth).max(1);
            for d in 0..local {
                let (x, y) = if horizontal {
                    (i, if inward == 1 { d } else { size - 1 - d })
                } else {
                    (if inward == 1 { d } else { size - 1 - d }, i)
                };
                if grid.in_bounds(x, y) && !grid.is_water(x, y) {
                    make_water(grid, x, y);
                    tiles.push(Point { x, y });
                }
            }
        }
        tiles
    };

    const OCEAN_CHANCE: f64 = 0.4;
    if rng.chance(OCEAN_CHANCE) {
        let t = gen_edge(grid, rng, true, 0, 1);
        if !t.is_empty() {
            edges.push(("north", t));
        }
    }
    if rng.chance(OCEAN_CHANCE) {
        let t = gen_edge(grid, rng, true, size - 1, -1);
        if !t.is_empty() {
            edges.push(("south", t));
        }
    }
    if rng.chance(OCEAN_CHANCE) {
        let t = gen_edge(grid, rng, false, size - 1, -1);
        if !t.is_empty() {
            edges.push(("east", t));
        }
    }
    if rng.chance(OCEAN_CHANCE) {
        let t = gen_edge(grid, rng, false, 0, 1);
        if !t.is_empty() {
            edges.push(("west", t));
        }
    }

    let mut bodies = Vec::new();
    for (side, tiles) in edges {
        let ax = tiles.iter().map(|p| p.x as f32).sum::<f32>() / tiles.len() as f32;
        let ay = tiles.iter().map(|p| p.y as f32).sum::<f32>() / tiles.len() as f32;
        bodies.push(WaterBody {
            id: format!("ocean-{side}-{}", bodies.len()),
            name: names::ocean_name(rng),
            kind: WaterBodyKind::Ocean,
            tiles,
            center_x: ax.round(),
            center_y: ay.round(),
        });
    }
    bodies
}

pub fn is_near_water(grid: &Grid, x: i32, y: i32, radius: i32) -> bool {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if grid.is_water(x + dx, y + dy) {
                return true;
            }
        }
    }
    false
}

/// Grass with lakes, oceans and scattered trees.
pub fn generate(size: i32, rng: &mut Rng) -> (Grid, Vec<WaterBody>) {
    let mut grid = Grid::new(size, |x, y| Tile::new(x, y, BuildingType::Grass));
    let seed = rng.next_f64() * 1000.0;
    let mut bodies = generate_lakes(&mut grid, rng, seed);
    bodies.extend(generate_oceans(&mut grid, rng, seed));
    for y in 0..size {
        for x in 0..size {
            if grid.is_water(x, y) {
                continue;
            }
            let tree_noise = perlin(x as f64 * 2.0, y as f64 * 2.0, seed + 500.0, 2);
            let is_tree = tree_noise > 0.72 && rng.next_f64() > 0.65;
            let near = is_near_water(&grid, x, y, 2) && rng.next_f64() > 0.7;
            if is_tree || near {
                grid.at_mut(x, y).building = Building::new(BuildingType::Tree);
            }
        }
    }
    (grid, bodies)
}
