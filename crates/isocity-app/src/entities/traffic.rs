//! Road traffic: cars, buses and emergency vehicles. Vehicles keep to the
//! right lane, queue behind each other and stop at red lights.

use super::bake::{BakeCache, Shape};
use super::paths;
use crate::iso::{tile_center, TILE_H, TILE_W};
use crate::render::tiles::TileMap;
use crate::render::world::View;
use gpui::Window;
use isocity_core::{BuildingType as B, Direction, GameState, Grid, Rng};

pub const CAR_COLORS: [u32; 5] = [0xd97777, 0xd4a01f, 0x2ba67a, 0x4d84c8, 0x9a6ac9];
pub const BUS_COLORS: [u32; 5] = [0xf59e0b, 0xef4444, 0x3b82f6, 0x10b981, 0x9333ea];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LightState {
    GreenNs,
    YellowNs,
    GreenEw,
    YellowEw,
}

pub fn light_state(t: f32) -> LightState {
    let c = t.rem_euclid(7.6);
    if c < 3.0 {
        LightState::GreenNs
    } else if c < 3.8 {
        LightState::YellowNs
    } else if c < 6.8 {
        LightState::GreenEw
    } else {
        LightState::YellowEw
    }
}

fn can_proceed(dir: Direction, s: LightState) -> bool {
    if dir.is_ns() {
        matches!(s, LightState::GreenNs | LightState::YellowNs)
    } else {
        matches!(s, LightState::GreenEw | LightState::YellowEw)
    }
}

/// Screen vector for one tile step in `dir`.
pub fn dir_vec(dir: Direction) -> (f32, f32) {
    match dir {
        Direction::North => (-TILE_W / 2.0, -TILE_H / 2.0),
        Direction::East => (TILE_W / 2.0, -TILE_H / 2.0),
        Direction::South => (TILE_W / 2.0, TILE_H / 2.0),
        Direction::West => (-TILE_W / 2.0, TILE_H / 2.0),
    }
}

pub fn dir_angle(dir: Direction) -> f32 {
    let (dx, dy) = dir_vec(dir);
    dy.atan2(dx)
}

pub fn dir_normal(dir: Direction) -> (f32, f32) {
    let (dx, dy) = dir_vec(dir);
    let l = (dx * dx + dy * dy).sqrt();
    (-dy / l, dx / l)
}

fn lane_offset(dir: Direction, base: f32) -> f32 {
    if matches!(dir, Direction::North | Direction::East) {
        base
    } else {
        -base
    }
}

/// World position of a vehicle on a tile.
pub fn vehicle_pos(x: i32, y: i32, dir: Direction, progress: f32, lane: f32) -> (f32, f32) {
    let (cx, cy) = tile_center(x as f32, y as f32);
    let (vx, vy) = dir_vec(dir);
    let (nx, ny) = dir_normal(dir);
    (cx + vx * progress + nx * lane, cy + vy * progress + ny * lane)
}

pub fn speed_multiplier(speed: u8) -> f32 {
    match speed {
        0 => 0.0,
        1 => 1.0,
        2 => 2.5,
        _ => 4.0,
    }
}

fn pick_next(grid: &Grid, rng: &mut Rng, prev: Direction, x: i32, y: i32) -> Option<Direction> {
    let opts = paths::road_options(grid, x, y);
    if opts.is_empty() {
        return None;
    }
    if grid.at(x, y).building.kind == B::Bridge && opts.contains(&prev) {
        return Some(prev);
    }
    let back = prev.opposite();
    let forward: Vec<Direction> = opts.iter().copied().filter(|d| *d != back).collect();
    let pool = if forward.is_empty() { &opts } else { &forward };
    rng.pick(pool).copied()
}

pub struct Car {
    id: u32,
    x: i32,
    y: i32,
    dir: Direction,
    progress: f32,
    speed: f32,
    age: f32,
    max_age: f32,
    color: u32,
    lane: f32,
}

pub struct Bus {
    x: i32,
    y: i32,
    dir: Direction,
    progress: f32,
    speed: f32,
    age: f32,
    max_age: f32,
    color: u32,
    lane: f32,
    path: Vec<(i32, i32)>,
    index: usize,
    stop_timer: f32,
    stops: Vec<(i32, i32)>,
}

pub struct Emergency {
    pub target: (i32, i32),
    x: i32,
    y: i32,
    dir: Direction,
    progress: f32,
    path: Vec<(i32, i32)>,
    index: usize,
    returning: bool,
    wait: f32,
    police: bool,
}

#[derive(Default)]
pub struct Traffic {
    pub cars: Vec<Car>,
    pub buses: Vec<Bus>,
    pub emergency: Vec<Emergency>,
    next_id: u32,
    car_timer: f32,
    bus_timer: f32,
    dispatch_timer: f32,
    road_tiles: Vec<(i32, i32)>,
    bus_stops: Vec<(i32, i32)>,
    fire_stations: Vec<(i32, i32)>,
    police_stations: Vec<(i32, i32)>,
    cached_version: u64,
}

impl Traffic {
    fn refresh(&mut self, state: &GameState) {
        if self.cached_version == state.game_version && !self.road_tiles.is_empty() {
            return;
        }
        self.cached_version = state.game_version;
        let g = &state.grid;
        self.road_tiles.clear();
        self.bus_stops.clear();
        self.fire_stations.clear();
        self.police_stations.clear();
        for t in g.iter() {
            if g.is_road(t.x, t.y) {
                self.road_tiles.push((t.x, t.y));
                // Stops next to commercial and civic buildings.
                let near = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| {
                    g.get(t.x + dx, t.y + dy).map_or(false, |n| {
                        n.zone == isocity_core::ZoneType::Commercial && !n.building.kind.is_terrain()
                            || matches!(n.building.kind, B::School | B::University | B::Hospital | B::CityHall | B::Stadium | B::Museum | B::RailStation | B::SubwayStation)
                    })
                });
                if near && (t.x * 7 + t.y * 3) % 3 == 0 {
                    self.bus_stops.push((t.x, t.y));
                }
            }
            if !t.building.under_construction() {
                match t.building.kind {
                    B::FireStation => self.fire_stations.push((t.x, t.y)),
                    B::PoliceStation => self.police_stations.push((t.x, t.y)),
                    _ => {}
                }
            }
        }
    }

    pub fn road_count(&self) -> usize {
        self.road_tiles.len()
    }

    pub fn update(&mut self, dt: f32, state: &GameState, map: &TileMap, light_time: f32, rng: &mut Rng, zoom: f32) {
        self.refresh(state);
        let g = &state.grid;
        let mult = speed_multiplier(state.speed);
        if zoom < 0.4 {
            self.cars.clear();
        } else {
            self.update_cars(dt, g, map, mult, light_time, rng);
        }
        self.update_buses(dt, g, state.stats.population, mult, rng);
        self.update_emergency(dt, state, mult);
    }

    fn update_cars(&mut self, dt: f32, g: &Grid, map: &TileMap, mult: f32, light_time: f32, rng: &mut Rng) {
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

    fn update_buses(&mut self, dt: f32, g: &Grid, population: i64, mult: f32, rng: &mut Rng) {
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

    fn update_emergency(&mut self, dt: f32, state: &GameState, mult: f32) {
        let g = &state.grid;
        self.dispatch_timer -= dt;
        if self.dispatch_timer <= 0.0 {
            self.dispatch_timer = 1.0;
            for t in g.iter().filter(|t| t.building.on_fire) {
                if self.emergency.iter().any(|e| e.target == (t.x, t.y)) || self.emergency.len() >= 12 {
                    continue;
                }
                let Some(goal) = paths::road_next_to(g, t.x, t.y) else {
                    continue;
                };
                let mut best: Option<Vec<(i32, i32)>> = None;
                for &(fx, fy) in &self.fire_stations {
                    if let Some(start) = paths::road_next_to(g, fx, fy) {
                        if let Some(p) = paths::road_path(g, start, goal) {
                            if best.as_ref().map_or(true, |b| p.len() < b.len()) {
                                best = Some(p);
                            }
                        }
                    }
                }
                if let Some(path) = best {
                    let (x, y) = path[0];
                    let dir = if path.len() > 1 {
                        Direction::from_step(path[1].0 - x, path[1].1 - y).unwrap_or(Direction::South)
                    } else {
                        Direction::South
                    };
                    self.emergency.push(Emergency {
                        target: (t.x, t.y),
                        x,
                        y,
                        dir,
                        progress: 0.0,
                        path,
                        index: 0,
                        returning: false,
                        wait: 0.0,
                        police: false,
                    });
                }
            }
            // Occasional police patrols.
            if self.emergency.iter().filter(|e| e.police).count() < (self.police_stations.len() * 2).min(8) && !self.road_tiles.is_empty() {
                let i = (state.tick as usize * 31 + self.road_tiles.len()) % self.road_tiles.len();
                let dest = self.road_tiles[i];
                for &(px, py) in &self.police_stations {
                    if let Some(start) = paths::road_next_to(g, px, py) {
                        if let Some(path) = paths::road_path(g, start, dest) {
                            if path.len() > 3 {
                                let dir = Direction::from_step(path[1].0 - path[0].0, path[1].1 - path[0].1).unwrap_or(Direction::South);
                                self.emergency.push(Emergency {
                                    target: (px, py),
                                    x: start.0,
                                    y: start.1,
                                    dir,
                                    progress: 0.0,
                                    path,
                                    index: 0,
                                    returning: true,
                                    wait: 0.0,
                                    police: true,
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }
        self.emergency.retain_mut(|e| {
            if e.wait > 0.0 {
                e.wait -= dt * mult;
                let burning = g.get(e.target.0, e.target.1).map_or(false, |t| t.building.on_fire);
                if burning && e.wait < 1.0 {
                    e.wait = 1.0;
                }
                if e.wait <= 0.0 {
                    if e.returning {
                        return false;
                    }
                    e.returning = true;
                    e.path.reverse();
                    e.index = 0;
                    e.progress = 0.0;
                }
                return true;
            }
            e.progress += 0.9 * dt * mult;
            while e.progress >= 1.0 {
                e.progress -= 1.0;
                e.index += 1;
                if e.index + 1 >= e.path.len() {
                    if e.returning {
                        return false;
                    }
                    e.wait = 3.0;
                    return true;
                }
                let (cx, cy) = e.path[e.index];
                e.x = cx;
                e.y = cy;
                let (nx, ny) = e.path[e.index + 1];
                if let Some(d) = Direction::from_step(nx - cx, ny - cy) {
                    e.dir = d;
                }
                if !g.is_road(cx, cy) {
                    return false;
                }
            }
            true
        });
    }

    pub fn paint(&self, window: &mut Window, v: &View, bake: &mut BakeCache, time: f32) {
        let zoom = v.cam.zoom;
        let (vx, vy, vw, vh) = v.world_rect();
        let visible = |x: f32, y: f32| x > vx - 40.0 && x < vx + vw + 40.0 && y > vy - 40.0 && y < vy + vh + 40.0;
        if zoom >= 0.4 {
            for c in &self.cars {
                let (x, y) = vehicle_pos(c.x, c.y, c.dir, c.progress, c.lane);
                if visible(x, y) {
                    bake.paint(window, v, Shape::Car(c.color), x, y, dir_angle(c.dir));
                }
            }
        }
        if zoom >= 0.35 {
            for b in &self.buses {
                let (x, y) = vehicle_pos(b.x, b.y, b.dir, b.progress, b.lane);
                if visible(x, y) {
                    bake.paint(window, v, Shape::Bus(b.color), x, y, dir_angle(b.dir));
                }
            }
        }
        for e in &self.emergency {
            let (x, y) = vehicle_pos(e.x, e.y, e.dir, e.progress, lane_offset(e.dir, 5.0));
            if !visible(x, y) {
                continue;
            }
            let shape = if e.police { Shape::PoliceCar } else { Shape::FireTruck };
            bake.paint(window, v, shape, x, y, dir_angle(e.dir));
            // Flashing light bar.
            let on = (time * 6.0) as i32 % 2 == 0;
            let (sx, sy) = v.to_screen(x, y - 3.0);
            let color = if on { 0xef4444 } else { 0x3b82f6 };
            crate::render::shapes::circle(window, sx, sy, 1.6 * zoom, crate::render::shapes::rgba(color, 0.95));
        }
    }
}
