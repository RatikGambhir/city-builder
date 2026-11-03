//! Pedestrians: residents leave home, path-find along sidewalks to shops,
//! workplaces and parks, spend time there and walk back.

use super::paths;
use super::traffic::{dir_normal, dir_vec, speed_multiplier};
use crate::iso::{tile_center, TILE_W};
use crate::render::shapes::{self, rgba};
use crate::render::world::View;
use gpui::Window;
use isocity_core::{BuildingType as B, Direction, GameState, Rng, ZoneType};

const SKIN: [u32; 8] = [0xffe4c4, 0xffd5b8, 0xffc8a8, 0xfdbf7e, 0xe0ac69, 0xc68642, 0x8d5524, 0x613318];
const SHIRT: [u32; 9] = [0xef4444, 0xf97316, 0xeab308, 0x22c55e, 0x3b82f6, 0x8b5cf6, 0xec4899, 0xffffff, 0x1f2937];
const PANTS: [u32; 6] = [0x1f2937, 0x374151, 0x4b5563, 0x1e3a8a, 0x7c2d12, 0x365314];
const HAT: [u32; 7] = [0xef4444, 0x3b82f6, 0x22c55e, 0xf97316, 0x8b5cf6, 0x1f2937, 0xffffff];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Walking,
    AtDestination,
    Idle,
}

pub struct Pedestrian {
    path: Vec<(i32, i32)>,
    index: usize,
    progress: f32,
    dir: Direction,
    side: f32,
    speed: f32,
    walk: f32,
    state: State,
    timer: f32,
    dest: (i32, i32),
    returning: bool,
    home: (i32, i32),
    offset: (f32, f32),
    skin: u32,
    shirt: u32,
    pants: u32,
    hat: Option<u32>,
    dog: bool,
    bag: bool,
    fade: f32,
    park: bool,
}

#[derive(Default)]
pub struct Pedestrians {
    pub people: Vec<Pedestrian>,
    homes: Vec<((i32, i32), (i32, i32))>,
    destinations: Vec<((i32, i32), (i32, i32), bool)>,
    road_count: usize,
    cached_version: u64,
    timer: f32,
}

impl Pedestrians {
    fn refresh(&mut self, state: &GameState) {
        if self.cached_version == state.game_version && (!self.homes.is_empty() || state.game_version == 0) {
            return;
        }
        self.cached_version = state.game_version;
        let g = &state.grid;
        self.homes.clear();
        self.destinations.clear();
        self.road_count = 0;
        for t in g.iter() {
            if g.is_road(t.x, t.y) {
                self.road_count += 1;
            }
            let k = t.building.kind;
            if k.is_terrain() || t.building.under_construction() {
                continue;
            }
            let Some(road) = paths::road_next_to(g, t.x, t.y) else {
                continue;
            };
            if t.zone == ZoneType::Residential || k == B::CabinHouse {
                self.homes.push(((t.x, t.y), road));
            } else if t.zone == ZoneType::Commercial || k.is_park() || matches!(k, B::School | B::University | B::Hospital | B::Museum | B::Stadium | B::CityHall | B::CommunityCenter | B::AmusementPark | B::SwimmingPool) {
                self.destinations.push(((t.x, t.y), road, k.is_park()));
            }
        }
    }

    pub fn update(&mut self, dt: f32, state: &GameState, rng: &mut Rng, zoom: f32) {
        self.refresh(state);
        if zoom < 0.5 {
            self.people.clear();
            return;
        }
        let mult = speed_multiplier(state.speed);
        let g = &state.grid;
        let max = ((self.road_count as f32 * 1.2) as usize).min(450);
        self.timer -= dt;
        if self.people.len() < max && self.timer <= 0.0 && !self.homes.is_empty() && !self.destinations.is_empty() {
            self.timer = 0.05;
            for _ in 0..6 {
                let (home, start) = *rng.pick(&self.homes).unwrap();
                let (dest, goal, park) = *rng.pick(&self.destinations).unwrap();
                if start == goal {
                    continue;
                }
                let Some(path) = paths::road_path(g, start, goal) else {
                    continue;
                };
                if path.len() < 2 || path.len() > 60 {
                    continue;
                }
                let dir = Direction::from_step(path[1].0 - path[0].0, path[1].1 - path[0].1).unwrap_or(Direction::South);
                self.people.push(Pedestrian {
                    path,
                    index: 0,
                    progress: rng.next_f32() * 0.5,
                    dir,
                    side: if rng.chance(0.5) { 1.0 } else { -1.0 },
                    speed: 0.12 + rng.next_f32() * 0.08,
                    walk: rng.next_f32() * 6.0,
                    state: State::Walking,
                    timer: 0.0,
                    dest,
                    returning: false,
                    home,
                    offset: ((rng.next_f32() - 0.5) * 14.0, (rng.next_f32() - 0.5) * 7.0),
                    skin: SKIN[rng.below(SKIN.len())],
                    shirt: SHIRT[rng.below(SHIRT.len())],
                    pants: PANTS[rng.below(PANTS.len())],
                    hat: if rng.chance(0.15) { Some(HAT[rng.below(HAT.len())]) } else { None },
                    dog: rng.chance(0.05),
                    bag: rng.chance(0.15),
                    fade: 0.0,
                    park,
                });
            }
        }
        self.people.retain_mut(|p| {
            p.fade = (p.fade + dt * 2.0).min(1.0);
            match p.state {
                State::AtDestination | State::Idle => {
                    p.timer -= dt * mult;
                    if p.state == State::AtDestination {
                        p.walk += dt * 3.0 * mult;
                    }
                    if p.timer <= 0.0 {
                        if p.state == State::Idle {
                            p.state = State::Walking;
                            return true;
                        }
                        if p.returning {
                            return false;
                        }
                        // Head home along the reversed path.
                        p.returning = true;
                        p.path.reverse();
                        p.index = 0;
                        p.progress = 0.0;
                        p.state = State::Walking;
                        let _ = p.home;
                        if p.path.len() >= 2 {
                            p.dir = Direction::from_step(p.path[1].0 - p.path[0].0, p.path[1].1 - p.path[0].1).unwrap_or(p.dir);
                        }
                    }
                    true
                }
                State::Walking => {
                    p.progress += p.speed * dt * mult;
                    p.walk += dt * 8.0 * mult;
                    if mult > 0.0 && rng.chance(0.0005) {
                        p.state = State::Idle;
                        p.timer = 1.0 + rng.next_f32() * 2.0;
                    }
                    while p.progress >= 1.0 {
                        p.progress -= 1.0;
                        p.index += 1;
                        if p.index + 1 >= p.path.len() {
                            if p.returning {
                                return false;
                            }
                            p.state = State::AtDestination;
                            p.timer = 15.0 + rng.next_f32() * 40.0;
                            p.progress = 0.0;
                            return true;
                        }
                        let (cx, cy) = p.path[p.index];
                        let (nx, ny) = p.path[p.index + 1];
                        if !g.is_road(cx, cy) {
                            return false;
                        }
                        if let Some(d) = Direction::from_step(nx - cx, ny - cy) {
                            p.dir = d;
                        }
                    }
                    true
                }
            }
        });
    }

    pub fn paint(&self, window: &mut Window, v: &View, recreation: bool) {
        let z = v.cam.zoom;
        if z < 0.5 {
            return;
        }
        let (vx, vy, vw, vh) = v.world_rect();
        for p in &self.people {
            let at_dest = p.state == State::AtDestination;
            if at_dest != recreation {
                continue;
            }
            let (x, y) = if at_dest {
                if !p.returning && p.park {
                    let (cx, cy) = tile_center(p.dest.0 as f32, p.dest.1 as f32);
                    (cx + p.offset.0, cy + p.offset.1)
                } else {
                    // Inside the building: not visible.
                    continue;
                }
            } else {
                let (tx, ty) = p.path[p.index.min(p.path.len() - 1)];
                let (cx, cy) = tile_center(tx as f32, ty as f32);
                let (dx, dy) = dir_vec(p.dir);
                let (nx, ny) = dir_normal(p.dir);
                let side = 10.0 * p.side;
                let yoff = match (p.dir, p.side > 0.0) {
                    (Direction::North, false) => -2.0,
                    (Direction::North, true) => -6.0,
                    (Direction::South, false) => 2.0,
                    (Direction::South, true) => 6.0,
                    (Direction::East, false) => -6.0,
                    (Direction::East, true) => -2.0,
                    (Direction::West, false) => 6.0,
                    (Direction::West, true) => 2.0,
                };
                (cx + dx * p.progress + nx * side, cy + dy * p.progress + ny * side + yoff)
            };
            if x < vx - 20.0 || x > vx + vw + 20.0 || y < vy - 20.0 || y > vy + vh + 20.0 {
                continue;
            }
            let (sx, sy) = v.to_screen(x, y);
            let s = 0.30 * z;
            let a = p.fade;
            let moving = p.state == State::Walking || at_dest;
            let bob = if moving { p.walk.sin() * 0.8 } else { 0.0 };
            let swing = if moving { p.walk.sin() * 3.0 } else { 0.0 };
            if z < 0.7 {
                shapes::circle(window, sx, sy - 5.0 * s, 3.0 * s, rgba(p.shirt, a));
                continue;
            }
            // Legs
            shapes::rect(window, sx + (-1.4 + swing * 0.3) * s, sy + (-1.0 + bob) * s, 1.3 * s, 6.0 * s, rgba(p.pants, a));
            shapes::rect(window, sx + (0.2 - swing * 0.3) * s, sy + (-1.0 + bob) * s, 1.3 * s, 6.0 * s, rgba(p.pants, a));
            // Body
            shapes::ellipse(window, sx, sy + (-5.0 + bob) * s, 2.5 * s, 4.0 * s, rgba(p.shirt, a));
            // Arms
            shapes::rect(window, sx + (-3.0 - swing * 0.2) * s, sy + (-6.0 + bob) * s, 1.0 * s, 4.0 * s, rgba(p.skin, a));
            shapes::rect(window, sx + (2.0 + swing * 0.2) * s, sy + (-6.0 + bob) * s, 1.0 * s, 4.0 * s, rgba(p.skin, a));
            // Head
            shapes::circle(window, sx, sy + (-12.0 + bob) * s, 3.0 * s, rgba(p.skin, a));
            if let Some(h) = p.hat {
                shapes::rect(window, sx - 3.2 * s, sy + (-15.5 + bob) * s, 6.4 * s, 2.2 * s, rgba(h, a));
            }
            if p.bag {
                shapes::rect(window, sx + 3.0 * s, sy + (-3.0 + bob) * s, 2.5 * s, 3.0 * s, rgba(0x92400e, a));
            }
            if p.dog && p.state == State::Walking {
                let ds = 0.26 * z;
                shapes::ellipse(window, sx + 8.0 * ds, sy + 6.0 * ds, 4.0 * ds, 2.0 * ds, rgba(0x8b4513, a));
                shapes::circle(window, sx + 12.0 * ds, sy + 4.0 * ds, 2.0 * ds, rgba(0x8b4513, a));
            }
        }
        let _ = TILE_W;
    }
}
