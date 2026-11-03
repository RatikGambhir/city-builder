//! Ambient effects: smoke from factories and power plants.

use super::traffic::speed_multiplier;
use crate::iso::{grid_to_world, TILE_H, TILE_W};
use crate::render::shapes::{self, rgba};
use crate::render::world::View;
use gpui::Window;
use isocity_core::{BuildingType as B, GameState, Rng};

struct Puff {
    x: f32,
    y: f32,
    vx: f32,
    age: f32,
    life: f32,
    size: f32,
}

#[derive(Default)]
pub struct Effects {
    puffs: Vec<Puff>,
    emitters: Vec<(f32, f32, f32)>,
    cached_version: u64,
    timer: f32,
}

impl Effects {
    fn refresh(&mut self, state: &GameState) {
        if self.cached_version == state.game_version && self.cached_version != 0 {
            return;
        }
        self.cached_version = state.game_version.max(1);
        self.emitters.clear();
        for t in state.grid.iter() {
            let b = &t.building;
            if b.under_construction() || b.abandoned {
                continue;
            }
            let rate = match b.kind {
                B::FactoryLarge => 1.0,
                B::FactoryMedium => 0.6,
                B::PowerPlant => 0.8,
                B::Warehouse => 0.2,
                _ => continue,
            };
            let fp = b.kind.size();
            let (sx, sy) = grid_to_world(t.x as f32 + (fp.width - 1) as f32 * 0.5, t.y as f32 + (fp.height - 1) as f32 * 0.5);
            let height = TILE_H * (1.2 + fp.width as f32 * 0.6);
            self.emitters.push((sx + TILE_W / 2.0, sy + TILE_H / 2.0 - height, rate));
        }
    }

    pub fn update(&mut self, dt: f32, state: &GameState, rng: &mut Rng, zoom: f32) {
        self.refresh(state);
        let m = speed_multiplier(state.speed).min(2.0);
        for p in &mut self.puffs {
            p.age += dt;
            p.y -= dt * 9.0;
            p.x += p.vx * dt;
        }
        self.puffs.retain(|p| p.age < p.life);
        if zoom < 0.35 || m == 0.0 {
            return;
        }
        self.timer -= dt;
        if self.timer <= 0.0 && self.puffs.len() < 1200 {
            self.timer = 0.15;
            for &(x, y, rate) in &self.emitters {
                if rng.chance((rate * 0.5) as f64) {
                    self.puffs.push(Puff {
                        x: x + (rng.next_f32() - 0.5) * 10.0,
                        y,
                        vx: 4.0 + rng.next_f32() * 6.0,
                        age: 0.0,
                        life: 3.0 + rng.next_f32() * 2.5,
                        size: 3.0 + rng.next_f32() * 3.0,
                    });
                }
            }
        }
    }

    pub fn paint(&self, window: &mut Window, v: &View) {
        let z = v.cam.zoom;
        if z < 0.35 {
            return;
        }
        let (vx, vy, vw, vh) = v.world_rect();
        for p in &self.puffs {
            if p.x < vx - 30.0 || p.x > vx + vw + 30.0 || p.y < vy - 30.0 || p.y > vy + vh + 30.0 {
                continue;
            }
            let k = p.age / p.life;
            let (sx, sy) = v.to_screen(p.x, p.y);
            let alpha = (1.0 - k) * 0.22 * (k * 6.0).min(1.0);
            let r = (p.size + k * 14.0) * z;
            shapes::circle(window, sx, sy, r, rgba(0xd1d5db, alpha * 0.6));
            shapes::circle(window, sx, sy, r * 0.65, rgba(0xe5e7eb, alpha));
        }
    }
}
