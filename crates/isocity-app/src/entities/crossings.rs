//! Railroad crossings: flashing signals and gates where rail crosses a road,
//! closing while a train approaches so road traffic waits (ported from the
//! web version's `railSystem.ts`).

use super::trains::Trains;
use crate::iso::grid_to_world;
use crate::render::shapes::{self, rgba};
use crate::render::tiles::TileMap;
use crate::render::world::View;
use gpui::Window;
use isocity_core::{BuildingType as B, GameState};
use std::collections::{HashMap, HashSet};

const GATE_SPEED: f32 = 180.0;
const FLASH_INTERVAL: f32 = 0.5;

const POLE: u32 = 0x374151;
const LIGHT_OFF: u32 = 0x1f2937;
const LIGHT_RED: u32 = 0xef4444;
const GATE_POLE: u32 = 0x4b5563;
const GATE_ARM: u32 = 0xfbbf24;
const GATE_STRIPE: u32 = 0x1f2937;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Orientation {
    Ns,
    Ew,
    Cross,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Corner {
    Nw,
    Ne,
    Sw,
    Se,
}

struct Crossing {
    x: i32,
    y: i32,
    orientation: Orientation,
    gate: f32,
    active: bool,
}

#[derive(Default)]
pub struct Crossings {
    list: Vec<Crossing>,
    /// Crossings road vehicles must not enter right now.
    pub closed: HashSet<(i32, i32)>,
    flash: f32,
    cached_version: u64,
}

fn rail_orientation(map: &TileMap, x: i32, y: i32) -> (bool, bool) {
    let ns = map.has_rail(x - 1, y) || map.has_rail(x + 1, y);
    let ew = map.has_rail(x, y - 1) || map.has_rail(x, y + 1);
    (ns, ew)
}

fn is_crossing(state: &GameState, map: &TileMap, x: i32, y: i32) -> Option<Orientation> {
    let t = state.grid.get(x, y)?;
    if t.building.kind != B::Road || !t.has_rail_overlay {
        return None;
    }
    let road = |x, y| state.grid.kind(x, y) == Some(B::Road);
    let road_ns = road(x - 1, y) || road(x + 1, y);
    let road_ew = road(x, y - 1) || road(x, y + 1);
    let (rail_ns, rail_ew) = rail_orientation(map, x, y);
    let orientation = match (rail_ns, rail_ew) {
        (true, true) => Orientation::Cross,
        (true, false) => Orientation::Ns,
        _ => Orientation::Ew,
    };
    let crossing = (road_ns && road_ew)
        || (rail_ns && rail_ew)
        || (road_ns && !road_ew && rail_ew && !rail_ns)
        || (road_ew && !road_ns && rail_ns && !rail_ew);
    crossing.then_some(orientation)
}

impl Crossings {
    fn refresh(&mut self, state: &GameState, map: &TileMap) {
        if self.cached_version == state.game_version && self.cached_version != 0 {
            return;
        }
        self.cached_version = state.game_version.max(1);
        let old: HashMap<(i32, i32), f32> = self.list.iter().map(|c| ((c.x, c.y), c.gate)).collect();
        self.list = state
            .grid
            .iter()
            .filter(|t| t.has_rail_overlay)
            .filter_map(|t| {
                is_crossing(state, map, t.x, t.y).map(|orientation| Crossing {
                    x: t.x,
                    y: t.y,
                    orientation,
                    gate: old.get(&(t.x, t.y)).copied().unwrap_or(0.0),
                    active: false,
                })
            })
            .collect();
    }

    pub fn update(&mut self, dt: f32, state: &GameState, map: &TileMap, trains: &Trains) {
        self.refresh(state, map);
        self.flash += dt;
        let mult = match state.speed {
            0 => 0.0,
            1 => 1.0,
            2 => 2.5,
            _ => 4.0,
        };
        self.closed.clear();
        for c in &mut self.list {
            let dist = trains.crossing_distance(c.x, c.y);
            c.active = dist.is_some();
            if c.active {
                self.closed.insert((c.x, c.y));
            }
            let target = if c.active { 90.0 } else { 0.0 };
            let step = GATE_SPEED * dt * mult;
            c.gate = if c.gate < target { (c.gate + step).min(target) } else { (c.gate - step).max(target) };
        }
    }

    pub fn paint(&self, window: &mut Window, v: &View) {
        let z = v.cam.zoom;
        if z < 0.5 {
            return;
        }
        let (vx, vy, vw, vh) = v.world_rect();
        for c in &self.list {
            let (x, y) = grid_to_world(c.x as f32, c.y as f32);
            if x + 64.0 < vx || x > vx + vw || y + 40.0 < vy || y - 30.0 > vy + vh {
                continue;
            }
            let corners: &[(Corner, Option<(f32, f32)>)] = match c.orientation {
                Orientation::Cross => &[(Corner::Ne, None), (Corner::Sw, None), (Corner::Nw, None), (Corner::Se, None)],
                Orientation::Ns => &[(Corner::Ne, Some((1.0, 30.0))), (Corner::Sw, Some((-1.0, -30.0)))],
                Orientation::Ew => &[(Corner::Nw, None), (Corner::Se, None)],
            };
            for &(corner, _) in corners {
                self.signal(window, v, x, y, corner, c.active);
            }
            if z >= 0.7 {
                for &(corner, ov) in corners {
                    gate(window, v, x, y, corner, c.gate, ov);
                }
            }
        }
    }

    fn signal(&self, window: &mut Window, v: &View, x: f32, y: f32, corner: Corner, active: bool) {
        let (w, h) = (crate::iso::TILE_W, crate::iso::TILE_H);
        let (ox, oy) = match corner {
            Corner::Nw => (w * 0.12, h * 0.48),
            Corner::Ne => (w * 0.52, h * 0.02),
            Corner::Sw => (w * 0.48, h * 0.98),
            Corner::Se => (w * 0.88, h * 0.52),
        };
        let z = v.cam.zoom;
        let scale = if z >= 0.8 { 1.0 } else { 0.85 } * 0.7;
        let pole = 14.0 * scale;
        let size = 2.5 * scale;
        let spacing = 5.0 * scale;
        let (sx, sy) = (x + ox, y + oy);
        let p0 = v.to_screen(sx, sy);
        let p1 = v.to_screen(sx, sy - pole);
        shapes::line(window, p0, p1, 2.0 * scale * z, rgba(POLE, 1.0));
        let ly = sy - pole + 4.0 * scale;
        let on = ((self.flash / FLASH_INTERVAL).floor() as i64) % 2 == 0;
        for (i, lit) in [(-0.5f32, active && on), (0.5, active && !on)] {
            let (cx, cy) = v.to_screen(sx + spacing * i, ly);
            if lit && z >= 0.7 {
                shapes::circle(window, cx, cy, size * 2.6 * z, rgba(LIGHT_RED, 0.3));
            }
            shapes::circle(window, cx, cy, size * z, rgba(if lit { LIGHT_RED } else { LIGHT_OFF }, 1.0));
        }
    }
}

fn gate(window: &mut Window, v: &View, x: f32, y: f32, corner: Corner, angle: f32, ov: Option<(f32, f32)>) {
    let (w, h) = (crate::iso::TILE_W, crate::iso::TILE_H);
    let (ox, oy, dir, tilt) = match corner {
        Corner::Nw => (w * 0.15, h * 0.50, 1.0, -30.0),
        Corner::Ne => (w * 0.55, h * 0.05, -1.0, -30.0),
        Corner::Sw => (w * 0.45, h * 0.95, 1.0, 30.0),
        Corner::Se => (w * 0.85, h * 0.50, -1.0, 30.0),
    };
    let (dir, tilt) = ov.unwrap_or((dir, tilt));
    let z = v.cam.zoom;
    let scale = if z >= 0.8 { 1.0 } else { 0.85 };
    let len = 18.0 * scale;
    let width = 1.1 * scale;
    let eff = (angle + angle / 90.0 * tilt).to_radians();
    let (gx, gy) = (x + ox, y + oy);
    let (ex, ey) = (gx + dir * eff.sin() * len, gy - eff.cos() * len);
    let (dx, dy) = (ex - gx, ey - gy);
    let l = (dx * dx + dy * dy).sqrt().max(0.001);
    let (ux, uy) = (dx / l, dy / l);
    let (nx, ny) = (-uy * width / 2.0, ux * width / 2.0);
    let seg = |a: f32, b: f32| {
        [
            v.to_screen(gx + ux * a + nx, gy + uy * a + ny),
            v.to_screen(gx + ux * b + nx, gy + uy * b + ny),
            v.to_screen(gx + ux * b - nx, gy + uy * b - ny),
            v.to_screen(gx + ux * a - nx, gy + uy * a - ny),
        ]
    };
    shapes::fill_poly(window, &seg(0.0, l), rgba(GATE_ARM, 1.0));
    let stripe = l / 8.0;
    for i in 0..4 {
        let a = stripe * (i * 2 + 1) as f32;
        shapes::fill_poly(window, &seg(a, a + stripe * 0.7), rgba(GATE_STRIPE, 1.0));
    }
    let (px, py) = v.to_screen(gx, gy);
    shapes::circle(window, px, py, width * 0.7 * z, rgba(GATE_POLE, 1.0));
}
