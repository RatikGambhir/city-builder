//! Private cars.

use crate::entities::paths;
use crate::render::tiles::TileMap;
use isocity_core::{Direction, Grid, Rng};
use std::collections::HashSet;
use super::Traffic;
use super::movement::{lane_offset, pick_next};
use super::signals::{can_proceed, light_state};

pub const CAR_COLORS: [u32; 5] = [0xd97777, 0xd4a01f, 0x2ba67a, 0x4d84c8, 0x9a6ac9];

pub struct Car {
    id: u32,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) dir: Direction,
    pub(super) progress: f32,
    pub(super) speed: f32,
    pub(super) age: f32,
    pub(super) max_age: f32,
    pub(super) color: u32,
    pub(super) lane: f32,
}

impl Traffic {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_cars(
        &mut self,
        dt: f32,
        g: &Grid,
        map: &TileMap,
        mult: f32,
        light_time: f32,
        closed: &HashSet<(i32, i32)>,
        rng: &mut Rng,
    ) {
        let max_cars = ((self.road_tiles.len() as f32 * 0.5) as usize).clamp(15, 500);
        self.car_timer -= dt;
        if self.cars.len() < max_cars && self.car_timer <= 0.0 && !self.road_tiles.is_empty() {
            let deficit = max_cars - self.cars.len();
            let n = if deficit > 50 { 4 } else { 2 }.min(deficit);
            let mut spawned = 0;
            for _ in 0..n {
                for _ in 0..20 {
                    let &(x, y) = rng.pick(&self.road_tiles).unwrap();
                    let opts = paths::road_options(g, x, y);
                    if let Some(&dir) = rng.pick(&opts) {
                        self.next_id += 1;
                        self.cars.push(Car {
                            id: self.next_id,
                            x,
                            y,
                            dir,
                            progress: rng.next_f32() * 0.8,
                            speed: (0.35 + rng.next_f32() * 0.35) * 0.7,
                            age: 0.0,
                            max_age: 45.0 + rng.next_f32() * 30.0,
                            color: CAR_COLORS[rng.below(CAR_COLORS.len())],
                            lane: lane_offset(dir, 4.0 + rng.next_f32() * 2.0),
                        });
                        spawned += 1;
                        break;
                    }
                }
            }
            self.car_timer = if spawned > 0 { 0.3 + rng.next_f32() * 0.4 } else { 0.1 };
        }
        let light = light_state(light_time);
        let size = g.size();
        let mut by_tile: std::collections::HashMap<i32, Vec<(u32, Direction, f32)>> = std::collections::HashMap::new();
        for c in &self.cars {
            by_tile.entry(c.y * size + c.x).or_default().push((c.id, c.dir, c.progress));
        }
        let mut cars = std::mem::take(&mut self.cars);
        cars.retain_mut(|car| {
            car.age += dt * mult;
            if car.age > car.max_age {
                return false;
            }
            if !g.is_road(car.x, car.y) {
                return false;
            }
            let (sx, sy) = car.dir.step();
            let (nx, ny) = (car.x + sx, car.y + sy);
            let mut stop = !map.is_intersection(car.x, car.y)
                && map.is_intersection(nx, ny)
                && car.progress > 0.35
                && !can_proceed(car.dir, light);
            // Railroad crossings: wait while a train approaches; a car
            // already well onto the crossing keeps going to clear it.
            if !stop {
                stop = (closed.contains(&(car.x, car.y)) && car.progress < 0.3)
                    || (car.progress > 0.5 && closed.contains(&(nx, ny)));
            }
            if !stop {
                if let Some(v) = by_tile.get(&(car.y * size + car.x)) {
                    stop = v.iter().any(|(id, d, p)| *id != car.id && *d == car.dir && *p > car.progress && p - car.progress < 0.25);
                }
            }
            if !stop && car.progress > 0.7 {
                if let Some(v) = by_tile.get(&(ny * size + nx)) {
                    stop = v.iter().any(|(_, d, p)| *d == car.dir && *p < 0.3);
                }
            }
            if !stop {
                car.progress += car.speed * dt * mult;
            }
            let mut guard = 0;
            while car.progress >= 1.0 && guard < 4 {
                guard += 1;
                if !paths::can_drive(g, car.x, car.y, car.dir) {
                    let opts = paths::road_options(g, car.x, car.y);
                    let others: Vec<Direction> = opts.iter().copied().filter(|d| *d != car.dir).collect();
                    let pool = if others.is_empty() { &opts } else { &others };
                    if let Some(&d) = rng.pick(pool) {
                        car.dir = d;
                        car.progress = 0.1;
                        car.lane = lane_offset(d, 4.0 + rng.next_f32() * 2.0);
                    } else {
                        car.progress = 0.5;
                    }
                    break;
                }
                let (sx, sy) = car.dir.step();
                car.x += sx;
                car.y += sy;
                car.progress -= 1.0;
                if let Some(d) = pick_next(g, rng, car.dir, car.x, car.y) {
                    if d != car.dir {
                        car.lane = lane_offset(d, 4.0 + rng.next_f32() * 2.0);
                    }
                    car.dir = d;
                }
            }
            true
        });
        self.cars = cars;
    }
}
