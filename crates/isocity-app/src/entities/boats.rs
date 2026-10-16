//! Boats sailing between marinas and piers, cargo barges crossing the ocean,
//! and the wakes they leave behind.

use super::bake::{BakeCache, Shape};
use super::paths;
use super::traffic::speed_multiplier;
use crate::iso::tile_center;
use crate::render::shapes::{self, rgba};
use crate::render::world::View;
use gpui::Window;
use isocity_core::{BuildingType as B, GameState, Grid, Rng};

const BOAT_COLORS: [u32; 6] = [0xffffff, 0x1e3a5f, 0x8b4513, 0x2f4f4f, 0xc41e3a, 0x1e90ff];
const BARGE_COLORS: [u32; 6] = [0x2c3e50, 0x34495e, 0x7f8c8d, 0xc0392b, 0x27ae60, 0x2980b9];

pub struct Vessel {
    path: Vec<(f32, f32)>,
    seg: usize,
    t: f32,
    speed: f32,
    shape: Shape,
    wait: f32,
    wake: Vec<(f32, f32, f32)>,
    wake_timer: f32,
    trips: u32,
    barge: bool,
}

impl Vessel {
    fn pos(&self) -> (f32, f32, f32) {
        let a = self.path[self.seg];
        let b = self.path[(self.seg + 1).min(self.path.len() - 1)];
        let x = a.0 + (b.0 - a.0) * self.t;
        let y = a.1 + (b.1 - a.1) * self.t;
        (x, y, (b.1 - a.1).atan2(b.0 - a.0))
    }
}

#[derive(Default)]
pub struct Boats {
    pub vessels: Vec<Vessel>,
    docks: Vec<(i32, i32)>,
    edge_water: Vec<(i32, i32)>,
    cached_version: u64,
    timer: f32,
    barge_timer: f32,
}

/// Water tile next to a dock building footprint.
fn dock_water(g: &Grid, x: i32, y: i32) -> Option<(i32, i32)> {
    let fp = g.get(x, y)?.building.kind.size();
    for dy in -1..=fp.height {
        for dx in -1..=fp.width {
            if g.is_water(x + dx, y + dy) {
                return Some((x + dx, y + dy));
            }
        }
    }
    None
}

/// Smooths a tile path into world points with small jitter.
fn to_points(path: &[(i32, i32)], rng: &mut Rng) -> Vec<(f32, f32)> {
    path.iter()
        .map(|&(x, y)| {
            let (cx, cy) = tile_center(x as f32, y as f32);
            (cx + (rng.next_f32() - 0.5) * 10.0, cy + (rng.next_f32() - 0.5) * 6.0)
        })
        .collect()
}

impl Boats {
    fn refresh(&mut self, state: &GameState) {
        if self.cached_version == state.game_version && self.cached_version != 0 {
            return;
        }
        self.cached_version = state.game_version.max(1);
        let g = &state.grid;
        self.docks = g
            .iter()
            .filter(|t| matches!(t.building.kind, B::MarinaDocksSmall | B::PierLarge) && !t.building.under_construction())
            .filter_map(|t| dock_water(g, t.x, t.y))
            .collect();
        let n = g.size();
        self.edge_water = g
            .iter()
            .filter(|t| (t.x == 0 || t.y == 0 || t.x == n - 1 || t.y == n - 1) && g.is_water(t.x, t.y))
            .map(|t| (t.x, t.y))
            .collect();
    }

    pub fn update(&mut self, dt: f32, state: &GameState, rng: &mut Rng) {
        self.refresh(state);
        let g = &state.grid;
        let m = speed_multiplier(state.speed);
        let boats = self.vessels.iter().filter(|v| !v.barge).count();
        let max_boats = (self.docks.len() * 6).min(60);
        self.timer -= dt;
        if self.docks.len() >= 1 && boats < max_boats && self.timer <= 0.0 {
            self.timer = 1.0 + rng.next_f32() * 2.0;
            let a = *rng.pick(&self.docks).unwrap();
            // Sail to another dock, or roam to a random spot of the same water body.
            let b = if self.docks.len() > 1 && rng.chance(0.6) {
                *rng.pick(&self.docks).unwrap()
            } else {
                let (dx, dy) = (rng.below(13) as i32 - 6, rng.below(13) as i32 - 6);
                (a.0 + dx, a.1 + dy)
            };
            if a != b && g.is_water(b.0, b.1) {
                if let Some(path) = paths::water_path(g, a, b) {
                    if path.len() >= 2 {
                        self.vessels.push(Vessel {
                            path: to_points(&path, rng),
                            seg: 0,
                            t: 0.0,
                            speed: 22.0 + rng.next_f32() * 12.0,
                            shape: Shape::Boat(BOAT_COLORS[rng.below(BOAT_COLORS.len())]),
                            wait: 0.0,
                            wake: Vec::new(),
                            wake_timer: 0.0,
                            trips: 0,
                            barge: false,
                        });
                    }
                }
            }
        }
        // Ocean barges travel from one map edge to another (or to a dock).
        let barges = self.vessels.iter().filter(|v| v.barge).count();
        self.barge_timer -= dt;
        if self.edge_water.len() >= 2 && barges < 4 && self.barge_timer <= 0.0 && state.stats.population > 1000 {
            self.barge_timer = 8.0 + rng.next_f32() * 12.0;
            let a = *rng.pick(&self.edge_water).unwrap();
            let b = if !self.docks.is_empty() && rng.chance(0.4) {
                *rng.pick(&self.docks).unwrap()
            } else {
                *rng.pick(&self.edge_water).unwrap()
            };
            let far = (a.0 - b.0).abs() + (a.1 - b.1).abs() > 10;
            if far {
                if let Some(path) = paths::water_path(g, a, b) {
                    self.vessels.push(Vessel {
                        path: to_points(&path, rng),
                        seg: 0,
                        t: 0.0,
                        speed: 8.0 + rng.next_f32() * 4.0,
                        shape: Shape::Barge(BARGE_COLORS[rng.below(BARGE_COLORS.len())]),
                        wait: 0.0,
                        wake: Vec::new(),
                        wake_timer: 0.0,
                        trips: 0,
                        barge: true,
                    });
                }
            }
        }
        self.vessels.retain_mut(|v| {
            for w in &mut v.wake {
                w.2 += dt;
            }
            v.wake.retain(|w| w.2 < 2.0);
            if v.wait > 0.0 {
                v.wait -= dt * m;
                if v.wait <= 0.0 {
                    if v.trips >= 2 || v.barge {
                        return false;
                    }
                    v.path.reverse();
                    v.seg = 0;
                    v.t = 0.0;
                    v.trips += 1;
                }
                return true;
            }
            let a = v.path[v.seg];
            let b = v.path[(v.seg + 1).min(v.path.len() - 1)];
            let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt().max(1.0);
            v.t += v.speed * dt * m / len;
            while v.t >= 1.0 {
                v.t -= 1.0;
                v.seg += 1;
                if v.seg + 1 >= v.path.len() {
                    v.seg = v.path.len() - 2;
                    v.t = 1.0;
                    v.wait = if v.barge { 8.0 } else { 5.0 + rng.next_f32() * 10.0 };
                    break;
                }
            }
            if m > 0.0 {
                v.wake_timer -= dt * m;
                if v.wake_timer <= 0.0 {
                    v.wake_timer = 0.06;
                    let (x, y, _) = v.pos();
                    v.wake.push((x, y, 0.0));
                }
            }
            true
        });
    }

    pub fn paint(&self, window: &mut Window, v: &View, bake: &mut BakeCache) {
        let z = v.cam.zoom;
        if z < 0.25 {
            return;
        }
        let (vx, vy, vw, vh) = v.world_rect();
        for b in &self.vessels {
            let (x, y, a) = b.pos();
            if x < vx - 60.0 || x > vx + vw + 60.0 || y < vy - 60.0 || y > vy + vh + 60.0 {
                continue;
            }
            let size = if b.barge { 3.0 } else { 1.5 };
            for &(wx, wy, age) in &b.wake {
                let (sx, sy) = v.to_screen(wx, wy);
                shapes::ellipse(window, sx, sy, (size + age * 3.0) * z, (size * 0.5 + age * 1.5) * z, rgba(0xffffff, (0.45 - age * 0.22).max(0.0)));
            }
            bake.paint(window, v, b.shape, x, y, a);
        }
    }
}
