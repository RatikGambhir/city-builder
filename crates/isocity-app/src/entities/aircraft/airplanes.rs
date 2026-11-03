//! Airplanes taking off, cruising and landing at airports.

use crate::iso::{TILE_H, TILE_W};
use isocity_core::{GameState, Rng};
use std::f32::consts::TAU;
use super::Aircraft;
use super::sprites::{PLANE_TYPES, PlaneType};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PlaneState {
    TakingOff,
    Flying,
    Landing,
}

pub struct Airplane {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) angle: f32,
    pub(super) state: PlaneState,
    pub(super) speed: f32,
    pub(super) altitude: f32,
    airport: (f32, f32),
    pub(super) contrail: Vec<(f32, f32, f32)>,
    contrail_timer: f32,
    life: f32,
    pub(super) kind: PlaneType,
}

impl Aircraft {
    pub(super) fn update_planes(&mut self, dt: f32, m: f32, state: &GameState, rng: &mut Rng) {
        let pop = state.stats.population;
        let n = state.grid.size() as f32;
        if self.airports.is_empty() || pop < 2000 {
            self.planes.clear();
        } else {
            let max = ((pop / 2000) as usize * 3).clamp(25, 80);
            self.plane_timer -= dt;
            if self.planes.len() < max && self.plane_timer <= 0.0 {
                self.plane_timer = 2.0 + rng.next_f32() * 5.0;
                let airport = *rng.pick(&self.airports).unwrap();
                let kind = *rng.pick(&PLANE_TYPES).unwrap();
                if rng.chance(0.5) {
                    self.planes.push(Airplane {
                        x: airport.0,
                        y: airport.1,
                        angle: rng.next_f32() * TAU,
                        state: PlaneState::TakingOff,
                        speed: 30.0 + rng.next_f32() * 20.0,
                        altitude: 0.0,
                        airport,
                        contrail: Vec::new(),
                        contrail_timer: 0.0,
                        life: 30.0 + rng.next_f32() * 20.0,
                        kind,
                    });
                } else {
                    let extent = n * TILE_W;
                    let (cx, cy) = (0.0, n * TILE_H / 2.0);
                    let (sx, sy) = match rng.below(4) {
                        0 => (cx + (rng.next_f32() - 0.5) * extent, cy - extent / 2.0 - 200.0),
                        1 => (cx + extent / 2.0 + 200.0, cy + (rng.next_f32() - 0.5) * extent / 2.0),
                        2 => (cx + (rng.next_f32() - 0.5) * extent, cy + extent / 2.0 + 200.0),
                        _ => (cx - extent / 2.0 - 200.0, cy + (rng.next_f32() - 0.5) * extent / 2.0),
                    };
                    self.planes.push(Airplane {
                        x: sx,
                        y: sy,
                        angle: (airport.1 - sy).atan2(airport.0 - sx),
                        state: PlaneState::Flying,
                        speed: 80.0 + rng.next_f32() * 40.0,
                        altitude: 1.0,
                        airport,
                        contrail: Vec::new(),
                        contrail_timer: 0.0,
                        life: 30.0 + rng.next_f32() * 20.0,
                        kind,
                    });
                }
            }
        }
        self.planes.retain_mut(|p| {
            for c in &mut p.contrail {
                c.2 += dt;
            }
            p.contrail.retain(|c| c.2 < 3.0);
            if p.altitude > 0.7 {
                p.contrail_timer += dt;
                while p.contrail_timer >= 0.02 {
                    p.contrail_timer -= 0.02;
                    p.contrail.push((p.x - p.angle.cos() * 40.0 * p.kind.scale() / 0.152, p.y - p.angle.sin() * 40.0 * p.kind.scale() / 0.152 + 8.0, 0.0));
                }
            }
            let step = p.speed * dt * m;
            match p.state {
                PlaneState::TakingOff => {
                    p.x += p.angle.cos() * step;
                    p.y += p.angle.sin() * step;
                    p.altitude = (p.altitude + dt * 0.3).min(1.0);
                    p.speed = (p.speed + dt * 20.0).min(120.0);
                    if p.altitude >= 1.0 {
                        p.state = PlaneState::Flying;
                    }
                }
                PlaneState::Flying => {
                    p.x += p.angle.cos() * step;
                    p.y += p.angle.sin() * step;
                    p.life -= dt;
                    let d = ((p.x - p.airport.0).powi(2) + (p.y - p.airport.1).powi(2)).sqrt();
                    if d < 400.0 && p.life < 10.0 {
                        p.state = PlaneState::Landing;
                    } else if p.life <= 0.0 {
                        return false;
                    }
                }
                PlaneState::Landing => {
                    p.angle = (p.airport.1 - p.y).atan2(p.airport.0 - p.x);
                    p.x += p.angle.cos() * step;
                    p.y += p.angle.sin() * step;
                    p.altitude = (p.altitude - dt * 0.25).max(0.0);
                    p.speed = (p.speed - dt * 15.0).max(30.0);
                    let d = ((p.x - p.airport.0).powi(2) + (p.y - p.airport.1).powi(2)).sqrt();
                    if d < 50.0 || p.altitude <= 0.0 {
                        return false;
                    }
                }
            }
            true
        });
    }
}
