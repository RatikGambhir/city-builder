//! Road traffic: cars, buses and emergency vehicles. Vehicles keep to the
//! right lane, queue behind each other and stop at red lights.

mod signals;
mod movement;
mod cars;
mod buses;
mod emergency;

pub use signals::{LightState, light_state};
pub use movement::{dir_angle, dir_normal, dir_vec, speed_multiplier, vehicle_pos};
pub use cars::Car;
pub use buses::Bus;
pub use emergency::Emergency;

use super::bake::{BakeCache, Shape};
use crate::render::tiles::TileMap;
use crate::render::world::View;
use gpui::Window;
use isocity_core::{BuildingType as B, GameState, Rng};
use std::collections::HashSet;
use movement::lane_offset;

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
                    g.get(t.x + dx, t.y + dy).is_some_and(|n| {
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

    /// `closed` holds railroad crossings that road vehicles must wait at.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        dt: f32,
        state: &GameState,
        map: &TileMap,
        light_time: f32,
        closed: &HashSet<(i32, i32)>,
        rng: &mut Rng,
        zoom: f32,
    ) {
        self.refresh(state);
        let g = &state.grid;
        let mult = speed_multiplier(state.speed);
        if zoom < 0.4 {
            self.cars.clear();
        } else {
            self.update_cars(dt, g, map, mult, light_time, closed, rng);
        }
        self.update_buses(dt, g, state.stats.population, mult, closed, rng);
        self.update_emergency(dt, state, mult);
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
