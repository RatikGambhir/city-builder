//! Seaplanes taxiing and flying from large lakes.

use crate::iso::tile_center;
use isocity_core::{GameState, Rng};
use std::f32::consts::{PI, TAU};
use std::sync::Arc;
use super::Aircraft;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SeaState {
    Taxi,
    TakeOff,
    Flying,
    Landing,
}

pub struct Seaplane {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) angle: f32,
    target_angle: f32,
    pub(super) state: SeaState,
    pub(super) altitude: f32,
    pub(super) speed: f32,
    timer: f32,
    bay: (f32, f32),
    bay_tiles: Arc<Vec<(i32, i32)>>,
    pub(super) wake: Vec<(f32, f32, f32)>,
    flights: u32,
}

impl Aircraft {
    pub(super) fn update_seaplanes(&mut self, dt: f32, m: f32, state: &GameState, rng: &mut Rng) {
        let pop = state.stats.population;
        if self.bays.is_empty() || pop < 3000 {
            self.seaplanes.clear();
        } else {
            self.sea_timer -= dt;
            if self.seaplanes.len() < (self.bays.len() * 2).min(25) && self.sea_timer <= 0.0 {
                self.sea_timer = 4.0 + rng.next_f32() * 6.0;
                let bay = rng.pick(&self.bays).unwrap().clone();
                let &(tx, ty) = rng.pick(&bay).unwrap();
                let (x, y) = tile_center(tx as f32, ty as f32);
                let a = rng.next_f32() * TAU;
                self.seaplanes.push(Seaplane {
                    x,
                    y,
                    angle: a,
                    target_angle: a,
                    state: SeaState::Taxi,
                    altitude: 0.0,
                    speed: 18.0,
                    timer: 4.0 + rng.next_f32() * 6.0,
                    bay: (x, y),
                    bay_tiles: bay,
                    wake: Vec::new(),
                    flights: 0,
                });
            }
        }
        let grid = &state.grid;
        self.seaplanes.retain_mut(|s| {
            for w in &mut s.wake {
                w.2 += dt;
            }
            s.wake.retain(|w| w.2 < 2.0);
            let on_water = |x: f32, y: f32| {
                let (gx, gy) = crate::iso::world_to_grid(x, y);
                grid.is_water(gx, gy)
            };
            match s.state {
                SeaState::Taxi => {
                    s.timer -= dt * m;
                    // Wander, steering back toward the bay if leaving water.
                    let ahead = (s.x + s.angle.cos() * 20.0, s.y + s.angle.sin() * 20.0);
                    if !on_water(ahead.0, ahead.1) {
                        s.target_angle = (s.bay.1 - s.y).atan2(s.bay.0 - s.x);
                    } else if rng.chance(0.01) {
                        s.target_angle += (rng.next_f32() - 0.5) * 1.5;
                    }
                    let mut d = s.target_angle - s.angle;
                    while d > PI {
                        d -= TAU;
                    }
                    while d < -PI {
                        d += TAU;
                    }
                    s.angle += d.clamp(-dt * 1.5, dt * 1.5);
                    s.x += s.angle.cos() * s.speed * dt * m;
                    s.y += s.angle.sin() * s.speed * dt * m;
                    s.wake.push((s.x, s.y, 0.0));
                    if s.timer <= 0.0 {
                        if s.flights >= 3 {
                            return false;
                        }
                        s.state = SeaState::TakeOff;
                        s.speed = 30.0;
                    }
                }
                SeaState::TakeOff => {
                    s.speed = (s.speed + dt * 30.0).min(90.0);
                    s.x += s.angle.cos() * s.speed * dt * m;
                    s.y += s.angle.sin() * s.speed * dt * m;
                    if s.speed > 55.0 {
                        s.altitude = (s.altitude + dt * 0.35).min(1.0);
                    } else {
                        s.wake.push((s.x, s.y, 0.0));
                    }
                    if s.altitude >= 1.0 {
                        s.state = SeaState::Flying;
                        s.timer = 25.0 + rng.next_f32() * 25.0;
                        s.flights += 1;
                    }
                }
                SeaState::Flying => {
                    s.timer -= dt * m;
                    if s.timer < 10.0 {
                        s.target_angle = (s.bay.1 - s.y).atan2(s.bay.0 - s.x);
                    } else if rng.chance(0.005) {
                        s.target_angle += (rng.next_f32() - 0.5) * 2.0;
                    }
                    let mut d = s.target_angle - s.angle;
                    while d > PI {
                        d -= TAU;
                    }
                    while d < -PI {
                        d += TAU;
                    }
                    s.angle += d.clamp(-dt * 0.8, dt * 0.8);
                    s.x += s.angle.cos() * s.speed * dt * m;
                    s.y += s.angle.sin() * s.speed * dt * m;
                    let dist = ((s.x - s.bay.0).powi(2) + (s.y - s.bay.1).powi(2)).sqrt();
                    if s.timer < 10.0 && dist < 150.0 {
                        s.state = SeaState::Landing;
                    }
                }
                SeaState::Landing => {
                    s.angle = (s.bay.1 - s.y).atan2(s.bay.0 - s.x);
                    s.speed = (s.speed - dt * 10.0).max(25.0);
                    s.x += s.angle.cos() * s.speed * dt * m;
                    s.y += s.angle.sin() * s.speed * dt * m;
                    s.altitude = (s.altitude - dt * 0.35).max(0.0);
                    if s.altitude <= 0.0 {
                        s.state = SeaState::Taxi;
                        s.speed = 18.0;
                        s.timer = 4.0 + rng.next_f32() * 6.0;
                        let &(tx, ty) = rng.pick(&s.bay_tiles).unwrap();
                        s.bay = tile_center(tx as f32, ty as f32);
                    }
                }
            }
            true
        });
    }
}
