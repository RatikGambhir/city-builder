//! Passenger and freight trains on the rail network. Each train follows a
//! route of rail tiles; carriages trail along smooth curves (matching the
//! track geometry) on the right-hand track.

use super::bake::{BakeCache, Shape};
use super::traffic::speed_multiplier;
use crate::iso::tile_center;
use crate::render::shapes::{self, rgba};
use crate::render::world::View;
use gpui::Window;
use isocity_core::{BuildingType as B, GameState, Grid, Rng};

const LOCO_COLORS: [u32; 6] = [0x1e40af, 0xdc2626, 0x059669, 0x7c3aed, 0xea580c, 0x0891b2];
const FREIGHT_COLORS: [u32; 6] = [0x8b4513, 0x696969, 0x2f4f4f, 0x8b0000, 0x006400, 0x4682b4];
const PASSENGER_COLORS: [u32; 4] = [0xc0c0c0, 0x1e40af, 0x059669, 0x7c3aed];
const TILE_LEN: f32 = 37.3;
const HALF_SEP: f32 = 64.0 * 0.22 / 2.0;

pub struct Train {
    route: Vec<(i32, i32)>,
    /// Position of the locomotive front in route tiles.
    pos: f32,
    speed: f32,
    cars: Vec<(Shape, f32)>,
    stop: f32,
    age: f32,
    last_station: (i32, i32),
    smoke: Vec<(f32, f32, f32)>,
    smoke_timer: f32,
}

#[derive(Default)]
pub struct Trains {
    pub trains: Vec<Train>,
    rail_tiles: Vec<(i32, i32)>,
    cached_version: u64,
    timer: f32,
}

fn is_station_adjacent(g: &Grid, x: i32, y: i32) -> bool {
    [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        match g.kind(nx, ny) {
            Some(B::RailStation) => true,
            Some(B::Empty) => isocity_core::sim::find_building_origin(g, nx, ny).map_or(false, |o| o.2 == B::RailStation),
            _ => false,
        }
    })
}

fn rail_neighbors(g: &Grid, x: i32, y: i32) -> Vec<(i32, i32)> {
    [(-1, 0), (0, -1), (1, 0), (0, 1)]
        .iter()
        .map(|(dx, dy)| (x + dx, y + dy))
        .filter(|&(nx, ny)| g.is_rail(nx, ny))
        .collect()
}

impl Train {
    /// Appends tiles so the route extends `ahead` tiles past `pos`.
    fn extend(&mut self, g: &Grid, rng: &mut Rng, ahead: usize) -> bool {
        while self.route.len() < self.pos as usize + ahead + 2 {
            let n = self.route.len();
            let cur = self.route[n - 1];
            let prev = if n >= 2 { Some(self.route[n - 2]) } else { None };
            let opts: Vec<(i32, i32)> = rail_neighbors(g, cur.0, cur.1).into_iter().filter(|p| Some(*p) != prev).collect();
            let next = if opts.is_empty() {
                // Dead end: reverse.
                match prev {
                    Some(p) => p,
                    None => return false,
                }
            } else if let Some(p) = prev {
                let straight = (2 * cur.0 - p.0, 2 * cur.1 - p.1);
                if opts.contains(&straight) && rng.chance(0.7) {
                    straight
                } else {
                    *rng.pick(&opts).unwrap()
                }
            } else {
                *rng.pick(&opts).unwrap()
            };
            self.route.push(next);
        }
        true
    }

    /// World position and heading at route coordinate `s`.
    fn sample(&self, s: f32) -> (f32, f32, f32) {
        let s = s.max(0.0);
        let k = (s.floor() as usize).min(self.route.len().saturating_sub(1));
        let f = s - k as f32;
        let c = |i: usize| {
            let (x, y) = self.route[i.min(self.route.len() - 1)];
            tile_center(x as f32, y as f32)
        };
        let mid = |a: (f32, f32), b: (f32, f32)| ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let pc = c(k);
        let a = if k == 0 { (2.0 * pc.0 - c(1).0, 2.0 * pc.1 - c(1).1) } else { c(k - 1) };
        let b = c(k + 1);
        let start = mid(a, pc);
        let end = mid(pc, b);
        // The coordinate runs from start (f=0) through the tile center to end.
        // Route coordinate k+0.5 is the tile center; remap so f in [0,1) spans
        // center(k) -> center(k+1) via the shared edge midpoint.
        let (p0, ctrl, p1, t) = if f < 0.5 {
            (start, pc, end, f + 0.5)
        } else {
            let nb = c(k + 2);
            (end, b, mid(b, nb), f - 0.5)
        };
        let u = 1.0 - t;
        let x = u * u * p0.0 + 2.0 * u * t * ctrl.0 + t * t * p1.0;
        let y = u * u * p0.1 + 2.0 * u * t * ctrl.1 + t * t * p1.1;
        let dx = 2.0 * u * (ctrl.0 - p0.0) + 2.0 * t * (p1.0 - ctrl.0);
        let dy = 2.0 * u * (ctrl.1 - p0.1) + 2.0 * t * (p1.1 - ctrl.1);
        let angle = dy.atan2(dx);
        // Right-hand track: offset to the right of travel.
        let l = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (rx, ry) = (-dy / l, dx / l);
        (x + rx * HALF_SEP, y + ry * HALF_SEP, angle)
    }
}

impl Trains {
    fn refresh(&mut self, state: &GameState) {
        if self.cached_version == state.game_version && !self.rail_tiles.is_empty() {
            return;
        }
        self.cached_version = state.game_version;
        let g = &state.grid;
        self.rail_tiles = g.iter().filter(|t| g.is_rail(t.x, t.y)).map(|t| (t.x, t.y)).collect();
    }

    pub fn update(&mut self, dt: f32, state: &GameState, rng: &mut Rng) {
        self.refresh(state);
        let g = &state.grid;
        if self.rail_tiles.len() < 6 {
            self.trains.clear();
            return;
        }
        let mult = speed_multiplier(state.speed);
        let max = (self.rail_tiles.len() / 6).clamp(1, 35);
        self.timer -= dt;
        if self.trains.len() < max && self.timer <= 0.0 {
            self.timer = 1.5;
            let start = *rng.pick(&self.rail_tiles).unwrap();
            let freight = rng.chance(0.45);
            let mut cars = Vec::new();
            let loco = LOCO_COLORS[rng.below(LOCO_COLORS.len())];
            cars.push((Shape::Locomotive(loco), 25.0));
            if freight {
                for _ in 0..(3 + rng.below(4)) {
                    let col = FREIGHT_COLORS[rng.below(FREIGHT_COLORS.len())];
                    let shape = match rng.below(3) {
                        0 => Shape::BoxCar(col),
                        1 => Shape::TankCar(col),
                        _ => Shape::FlatCar(col),
                    };
                    cars.push((shape, 28.0));
                }
                cars.push((Shape::Caboose(0xb91c1c), 16.0));
            } else {
                let col = PASSENGER_COLORS[rng.below(PASSENGER_COLORS.len())];
                for _ in 0..(2 + rng.below(3)) {
                    cars.push((Shape::PassengerCar(col), 16.0));
                }
            }
            let mut t = Train {
                route: vec![start],
                pos: 0.0,
                speed: if freight { 0.9 } else { 1.2 },
                cars,
                stop: 0.0,
                age: 0.0,
                last_station: (-1, -1),
                smoke: Vec::new(),
                smoke_timer: 0.0,
            };
            if t.extend(g, rng, 12) {
                // Start with the whole train on the track.
                t.pos = t.cars.iter().map(|c| (c.1 + 3.0) / TILE_LEN).sum::<f32>().min((t.route.len() - 3) as f32);
                self.trains.push(t);
            }
        }
        self.trains.retain_mut(|t| {
            t.age += dt * mult;
            for p in &mut t.smoke {
                p.2 += dt;
                p.1 -= dt * 10.0;
            }
            t.smoke.retain(|p| p.2 < 2.5);
            if t.age > 120.0 {
                return false;
            }
            let cur = t.route[(t.pos as usize).min(t.route.len() - 1)];
            if !g.is_rail(cur.0, cur.1) {
                return false;
            }
            if t.stop > 0.0 {
                t.stop -= dt * mult;
                return true;
            }
            if is_station_adjacent(g, cur.0, cur.1) && t.last_station != cur {
                t.last_station = cur;
                t.stop = 2.0;
            }
            t.pos += t.speed * dt * mult;
            if !t.extend(g, rng, 6) {
                return false;
            }
            // Smoke from the locomotive.
            t.smoke_timer -= dt * mult;
            if t.smoke_timer <= 0.0 && mult > 0.0 {
                t.smoke_timer = 0.12;
                let (x, y, _) = t.sample(t.pos - 0.2);
                t.smoke.push((x, y - 8.0, 0.0));
            }
            // Drop route history far behind the last carriage.
            let train_len: f32 = t.cars.iter().map(|c| (c.1 + 3.0) / TILE_LEN).sum();
            let drop = (t.pos - train_len - 3.0).floor();
            if drop > 8.0 {
                let d = drop as usize;
                t.route.drain(..d);
                t.pos -= d as f32;
            }
            true
        });
    }

    pub fn paint(&self, window: &mut Window, v: &View, bake: &mut BakeCache) {
        if v.cam.zoom < 0.2 {
            return;
        }
        let (vx, vy, vw, vh) = v.world_rect();
        for t in &self.trains {
            let mut s = t.pos;
            for (shape, len) in &t.cars {
                let center = s - (len / 2.0) / TILE_LEN;
                let (x, y, a) = t.sample(center);
                if x > vx - 40.0 && x < vx + vw + 40.0 && y > vy - 40.0 && y < vy + vh + 40.0 {
                    bake.paint(window, v, *shape, x, y, a);
                }
                s -= (len + 3.0) / TILE_LEN;
            }
            for &(x, y, age) in &t.smoke {
                let (sx, sy) = v.to_screen(x, y);
                let r = (2.0 + age * 3.0) * v.cam.zoom;
                shapes::circle(window, sx, sy, r, rgba(0xd1d5db, (0.5 - age * 0.2).max(0.0)));
            }
        }
    }

    /// True if any train is within `radius` tiles of `(x, y)` (crossings).
    pub fn near(&self, x: i32, y: i32, radius: i32) -> bool {
        self.trains.iter().any(|t| {
            let i = (t.pos as usize).min(t.route.len() - 1);
            let lo = i.saturating_sub(6);
            let hi = (i + radius as usize).min(t.route.len() - 1);
            t.route[lo..=hi].iter().any(|&(rx, ry)| (rx - x).abs() + (ry - y).abs() <= 1)
        })
    }
}
