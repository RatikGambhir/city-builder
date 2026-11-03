//! Buses.

use crate::entities::paths;
use isocity_core::{Direction, Grid, Rng};
use std::collections::HashSet;
use super::Traffic;
use super::movement::lane_offset;

pub const BUS_COLORS: [u32; 5] = [0xf59e0b, 0xef4444, 0x3b82f6, 0x10b981, 0x9333ea];

pub struct Bus {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) dir: Direction,
    pub(super) progress: f32,
    pub(super) speed: f32,
    pub(super) age: f32,
    pub(super) max_age: f32,
    pub(super) color: u32,
    pub(super) lane: f32,
    pub(super) path: Vec<(i32, i32)>,
    pub(super) index: usize,
    stop_timer: f32,
    stops: Vec<(i32, i32)>,
}

impl Traffic {
    pub(super) fn update_buses(&mut self, dt: f32, g: &Grid, population: i64, mult: f32, closed: &HashSet<(i32, i32)>, rng: &mut Rng) {
        if population < 600 || self.bus_stops.len() < 2 {
            self.buses.clear();
            return;
        }
        self.bus_timer -= dt;
        if self.buses.len() < 35 && self.bus_timer <= 0.0 {
            self.bus_timer = 2.5 + rng.next_f32() * 3.5;
            for _ in 0..10 {
                let a = *rng.pick(&self.bus_stops).unwrap();
                let b = *rng.pick(&self.bus_stops).unwrap();
                if a == b {
                    continue;
                }
                if let Some(path) = paths::road_path(g, a, b) {
                    if path.len() < 2 {
                        continue;
                    }
                    let dir = Direction::from_step(path[1].0 - path[0].0, path[1].1 - path[0].1).unwrap_or(Direction::South);
                    self.buses.push(Bus {
                        x: a.0,
                        y: a.1,
                        dir,
                        progress: 0.0,
                        speed: 0.3 + rng.next_f32() * 0.15,
                        age: 0.0,
                        max_age: 160.0 + rng.next_f32() * 120.0,
                        color: BUS_COLORS[rng.below(BUS_COLORS.len())],
                        lane: lane_offset(dir, 6.0 + rng.next_f32() * 2.5),
                        path,
                        index: 0,
                        stop_timer: 0.0,
                        stops: vec![a, b],
                    });
                    break;
                }
            }
        }
        let stops = self.bus_stops.clone();
        self.buses.retain_mut(|bus| {
            bus.age += dt * mult;
            if bus.age > bus.max_age || !g.is_road(bus.x, bus.y) {
                return false;
            }
            if bus.stop_timer > 0.0 {
                bus.stop_timer -= dt * mult;
                return true;
            }
            let next = bus.path.get(bus.index + 1).copied();
            if (closed.contains(&(bus.x, bus.y)) && bus.progress < 0.3)
                || (bus.progress > 0.5 && next.is_some_and(|n| closed.contains(&n)))
            {
                return true;
            }
            bus.progress += bus.speed * dt * mult;
            while bus.progress >= 1.0 {
                bus.progress -= 1.0;
                bus.index += 1;
                if bus.index + 1 >= bus.path.len() {
                    // Arrived: stop, then route to a new destination.
                    let here = (bus.x + bus.dir.step().0, bus.y + bus.dir.step().1);
                    bus.x = here.0;
                    bus.y = here.1;
                    bus.stop_timer = 1.6 + rng.next_f32() * 1.9;
                    bus.progress = 0.0;
                    let dest = *rng.pick(&stops).unwrap();
                    match paths::road_path(g, here, dest) {
                        Some(p) if p.len() >= 2 => {
                            bus.dir = Direction::from_step(p[1].0 - p[0].0, p[1].1 - p[0].1).unwrap_or(bus.dir);
                            bus.lane = lane_offset(bus.dir, bus.lane.abs());
                            bus.path = p;
                            bus.index = 0;
                            bus.stops = vec![here, dest];
                        }
                        _ => return false,
                    }
                    return true;
                }
                let (cx, cy) = bus.path[bus.index];
                bus.x = cx;
                bus.y = cy;
                let (nx, ny) = bus.path[bus.index + 1];
                if let Some(d) = Direction::from_step(nx - cx, ny - cy) {
                    if d != bus.dir {
                        bus.lane = lane_offset(d, bus.lane.abs());
                    }
                    bus.dir = d;
                }
                if bus.stops.contains(&(cx, cy)) && bus.index > 0 {
                    bus.stop_timer = 1.6 + rng.next_f32() * 1.9;
                }
            }
            true
        });
    }
}
