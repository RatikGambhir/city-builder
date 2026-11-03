//! Airplanes (sprite sheet, with contrails and shadows), helicopters that
//! shuttle between heliports, and seaplanes operating from large lakes.

mod sprites;
mod airplanes;
mod helicopters;
mod seaplanes;

pub use sprites::{PlaneSprites, PlaneType};
pub use airplanes::Airplane;
pub use helicopters::Helicopter;
pub use seaplanes::Seaplane;

use super::bake::{BakeCache, Shape};
use super::paths;
use crate::iso::{grid_to_world, tile_center, TILE_H, TILE_W};
use crate::render::shapes::{self, rgba};
use crate::render::world::View;
use crate::sprites::SheetStore;
use gpui::Window;
use isocity_core::{BuildingType as B, GameState, Rng};
use std::f32::consts::PI;
use std::sync::Arc;

#[derive(Default)]
pub struct Aircraft {
    pub planes: Vec<Airplane>,
    pub helis: Vec<Helicopter>,
    pub seaplanes: Vec<Seaplane>,
    sprites: PlaneSprites,
    airports: Vec<(f32, f32)>,
    heliports: Vec<(f32, f32)>,
    bays: Vec<Arc<Vec<(i32, i32)>>>,
    cached_version: u64,
    plane_timer: f32,
    heli_timer: f32,
    sea_timer: f32,
}

fn mult(speed: u8) -> f32 {
    match speed {
        0 => 0.0,
        1 => 1.0,
        2 => 1.5,
        _ => 2.0,
    }
}

impl Aircraft {
    fn refresh(&mut self, state: &GameState) {
        if self.cached_version == state.game_version && self.cached_version != 0 {
            return;
        }
        self.cached_version = state.game_version.max(1);
        let g = &state.grid;
        self.airports.clear();
        self.heliports.clear();
        for t in g.iter() {
            if t.building.under_construction() || t.building.abandoned {
                continue;
            }
            let (sx, sy) = grid_to_world(t.x as f32, t.y as f32);
            match t.building.kind {
                B::Airport => {
                    self.airports.push((sx + TILE_W * 0.5 + 0.0, sy + TILE_H * 2.0));
                    self.heliports.push((sx + TILE_W * 0.5, sy + TILE_H * 2.0));
                }
                B::Hospital | B::PoliceStation => {
                    let fp = t.building.kind.size();
                    let (cx, cy) = tile_center(t.x as f32 + (fp.width - 1) as f32 / 2.0, t.y as f32 + (fp.height - 1) as f32 / 2.0);
                    self.heliports.push((cx, cy - 10.0));
                }
                _ => {}
            }
        }
        // Large lakes/bays for seaplanes.
        self.bays.clear();
        let mut seen = std::collections::HashSet::new();
        for wb in &state.water_bodies {
            let tiles: Vec<(i32, i32)> = wb.tiles.iter().map(|p| (p.x, p.y)).filter(|&(x, y)| g.is_water(x, y)).collect();
            if tiles.len() >= 12 && seen.insert(wb.id.clone()) {
                self.bays.push(Arc::new(tiles));
            }
        }
        if self.bays.is_empty() {
            // Imported cities may lack water body metadata: find a big lake.
            for t in g.iter() {
                if g.is_water(t.x, t.y) && paths::water_body_size(g, t.x, t.y, 40) >= 12 {
                    let tiles: Vec<(i32, i32)> = g
                        .iter()
                        .filter(|o| g.is_water(o.x, o.y) && (o.x - t.x).abs() < 8 && (o.y - t.y).abs() < 8)
                        .map(|o| (o.x, o.y))
                        .collect();
                    self.bays.push(Arc::new(tiles));
                    break;
                }
            }
        }
    }

    pub fn update(&mut self, dt: f32, state: &GameState, rng: &mut Rng) {
        self.refresh(state);
        let m = mult(state.speed);
        if m == 0.0 {
            return;
        }
        self.update_planes(dt, m, state, rng);
        self.update_helis(dt, m, state, rng);
        self.update_seaplanes(dt, m, state, rng);
    }

    /// Wakes and taxiing seaplanes (water level).
    pub fn paint_water_level(&mut self, window: &mut Window, v: &View, store: &SheetStore) {
        let z = v.cam.zoom;
        for s in &self.seaplanes {
            for &(x, y, age) in &s.wake {
                let (sx, sy) = v.to_screen(x, y);
                shapes::circle(window, sx, sy, (1.0 + age * 2.0) * z, rgba(0xffffff, (0.35 - age * 0.17).max(0.0)));
            }
        }
        let taxiing: Vec<(f32, f32, f32)> = self.seaplanes.iter().filter(|s| s.altitude < 0.05).map(|s| (s.x, s.y, s.angle)).collect();
        for (x, y, a) in taxiing {
            self.sprites.paint(window, v, store, PlaneType::Seaplane, x, y, a, 0.7);
        }
    }

    pub fn paint_air(&mut self, window: &mut Window, v: &View, store: &SheetStore, bake: &mut BakeCache) {
        let z = v.cam.zoom;
        let (vx, vy, vw, vh) = v.world_rect();
        let vis = |x: f32, y: f32, m: f32| x > vx - m && x < vx + vw + m && y > vy - m && y < vy + vh + m;
        // Contrails first, behind every plane.
        for p in &self.planes {
            for &(x, y, age) in &p.contrail {
                if !vis(x, y, 20.0) {
                    continue;
                }
                let op = (1.0 - age / 3.0).max(0.0) * 0.4 * p.altitude;
                let (sx, sy) = v.to_screen(x, y);
                shapes::circle(window, sx, sy, (3.0 + age * 8.0) * z, rgba(0xffffff, op));
            }
        }
        let planes: Vec<(PlaneType, f32, f32, f32, f32)> = self.planes.iter().map(|p| (p.kind, p.x, p.y, p.angle, p.altitude)).collect();
        for (kind, x, y, a, alt) in planes {
            if !vis(x, y, 80.0) {
                continue;
            }
            if alt < 0.8 {
                let off = (1.0 - alt) * 18.0;
                let (sx, sy) = v.to_screen(x + off, y + off * 0.5);
                let s = (0.55 + alt * 0.35) * kind.scale() / 0.152;
                shapes::ellipse(window, sx, sy, 50.0 * 0.152 * s * z, 20.0 * 0.152 * s * 0.5 * z, rgba(0x000000, 0.25 * (1.0 - alt)));
            }
            self.sprites.paint(window, v, store, kind, x, y, a, 0.7 + alt * 0.5);
        }
        for h in &self.helis {
            if !vis(h.x, h.y, 40.0) {
                continue;
            }
            let lift = h.altitude * 40.0;
            let (shx, shy) = v.to_screen(h.x + lift * 0.4, h.y + lift * 0.2);
            shapes::ellipse(window, shx, shy, 7.0 * z, 3.0 * z, rgba(0x000000, 0.2));
            let y = h.y - lift;
            bake.paint(window, v, Shape::Helicopter(h.color), h.x, y, h.angle);
            let (sx, sy) = v.to_screen(h.x, y);
            let r = 11.0 * z;
            for k in 0..2 {
                let a = h.rotor + k as f32 * PI / 2.0;
                let (dx, dy) = (a.cos() * r, a.sin() * r * 0.5);
                shapes::line(window, (sx - dx, sy - dy), (sx + dx, sy + dy), 1.2 * z, rgba(0x1f2937, 0.7));
            }
        }
        let flying: Vec<(f32, f32, f32, f32)> = self.seaplanes.iter().filter(|s| s.altitude >= 0.05).map(|s| (s.x, s.y, s.angle, s.altitude)).collect();
        for (x, y, a, alt) in flying {
            if vis(x, y, 60.0) {
                self.sprites.paint(window, v, store, PlaneType::Seaplane, x, y, a, 0.7 + alt * 0.5);
            }
        }
    }
}
