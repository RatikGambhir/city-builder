//! Helicopters shuttling between heliports.

use isocity_core::{GameState, Rng};
use super::Aircraft;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HeliState {
    TakingOff,
    Flying,
    Landing,
}

pub struct Helicopter {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) angle: f32,
    pub(super) state: HeliState,
    pub(super) altitude: f32,
    dest: (f32, f32),
    pub(super) rotor: f32,
    pub(super) color: u32,
}

impl Aircraft {
    pub(super) fn update_helis(&mut self, dt: f32, m: f32, state: &GameState, rng: &mut Rng) {
        let pop = state.stats.population;
        if self.heliports.len() < 2 || pop < 3000 {
            self.helis.clear();
        } else {
            let max = (self.heliports.len() * 2).clamp(2, 12);
            self.heli_timer -= dt;
            if self.helis.len() < max && self.heli_timer <= 0.0 {
                self.heli_timer = 3.0 + rng.next_f32() * 4.0;
                let a = *rng.pick(&self.heliports).unwrap();
                let b = *rng.pick(&self.heliports).unwrap();
                if a != b {
                    self.helis.push(Helicopter {
                        x: a.0,
                        y: a.1,
                        angle: (b.1 - a.1).atan2(b.0 - a.0),
                        state: HeliState::TakingOff,
                        altitude: 0.0,
                        dest: b,
                        rotor: 0.0,
                        color: [0xdc2626, 0xffffff, 0x1e3a8a, 0xf97316, 0x059669][rng.below(5)],
                    });
                }
            }
        }
        self.helis.retain_mut(|h| {
            h.rotor += dt * 30.0;
            match h.state {
                HeliState::TakingOff => {
                    h.altitude = (h.altitude + dt * 0.5).min(0.5);
                    if h.altitude >= 0.5 {
                        h.state = HeliState::Flying;
                    }
                }
                HeliState::Flying => {
                    let (dx, dy) = (h.dest.0 - h.x, h.dest.1 - h.y);
                    let d = (dx * dx + dy * dy).sqrt();
                    h.angle = dy.atan2(dx);
                    let step = 60.0 * dt * m;
                    if d <= step + 5.0 {
                        h.state = HeliState::Landing;
                    } else {
                        h.x += dx / d * step;
                        h.y += dy / d * step;
                    }
                }
                HeliState::Landing => {
                    h.altitude -= dt * 0.4;
                    if h.altitude <= 0.0 {
                        return false;
                    }
                }
            }
            true
        });
    }
}
