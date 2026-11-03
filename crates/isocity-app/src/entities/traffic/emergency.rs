//! Fire trucks and police cars responding to incidents.

use crate::entities::paths;
use isocity_core::{Direction, GameState};
use super::Traffic;

pub struct Emergency {
    pub target: (i32, i32),
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) dir: Direction,
    pub(super) progress: f32,
    pub(super) path: Vec<(i32, i32)>,
    pub(super) index: usize,
    returning: bool,
    wait: f32,
    pub(super) police: bool,
}

impl Traffic {
    pub(super) fn update_emergency(&mut self, dt: f32, state: &GameState, mult: f32) {
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
                            if best.as_ref().is_none_or(|b| p.len() < b.len()) {
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
                let burning = g.get(e.target.0, e.target.1).is_some_and(|t| t.building.on_fire);
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
}
