//! Animated, purely visual inhabitants of the city. These run client-side
//! (like the web version) and are not part of the shared simulation state.

pub mod aircraft;
pub mod bake;
pub mod boats;
pub mod crossings;
pub mod effects;
pub mod paths;
pub mod pedestrians;
pub mod traffic;
pub mod trains;

use crate::render::world::{View, WorldRenderer};
use gpui::Window;
use isocity_core::{GameState, Rng};

pub struct Entities {
    pub traffic_time: f32,
    pub traffic: traffic::Traffic,
    pub pedestrians: pedestrians::Pedestrians,
    pub trains: trains::Trains,
    pub aircraft: aircraft::Aircraft,
    pub boats: boats::Boats,
    pub crossings: crossings::Crossings,
    pub effects: effects::Effects,
    pub bake: bake::BakeCache,
    rng: Rng,
}

impl Entities {
    pub fn new(_grid_size: i32) -> Self {
        Self {
            traffic_time: 0.0,
            traffic: Default::default(),
            pedestrians: Default::default(),
            trains: Default::default(),
            aircraft: Default::default(),
            boats: Default::default(),
            crossings: Default::default(),
            effects: Default::default(),
            bake: Default::default(),
            rng: Rng::from_entropy(),
        }
    }

    pub fn update(&mut self, dt: f32, state: &GameState, v: &View, renderer: &mut WorldRenderer) {
        if state.speed > 0 {
            self.traffic_time += dt * traffic::speed_multiplier(state.speed).min(2.5);
        }
        let zoom = v.cam.zoom;
        self.trains.update(dt, state, &mut self.rng);
        if let Some(map) = renderer.tile_map.as_ref() {
            self.crossings.update(dt, state, map, &self.trains);
            let closed = &self.crossings.closed;
            self.traffic.update(dt, state, map, self.traffic_time, closed, &mut self.rng, zoom);
        }
        self.pedestrians.update(dt, state, &mut self.rng, zoom);
        self.aircraft.update(dt, state, &mut self.rng);
        self.boats.update(dt, state, &mut self.rng);
        self.effects.update(dt, state, &mut self.rng, zoom);
    }

    /// Things drawn below building sprites: boats, vehicles, trains, walkers.
    pub fn paint_ground_level(&mut self, window: &mut Window, v: &View, _state: &GameState, renderer: &mut WorldRenderer, time: f32) {
        self.boats.paint(window, v, &mut self.bake);
        let store = renderer.sprites.store.clone();
        self.aircraft.paint_water_level(window, v, &store);
        self.traffic.paint(window, v, &mut self.bake, time);
        self.trains.paint(window, v, &mut self.bake);
        self.pedestrians.paint(window, v, false);
    }

    /// Things drawn over buildings: people in parks and smoke.
    pub fn paint_above_buildings(&mut self, window: &mut Window, v: &View, _state: &GameState, _renderer: &mut WorldRenderer, _time: f32) {
        self.pedestrians.paint(window, v, true);
        self.effects.paint(window, v);
    }

    /// Aircraft fly above everything.
    pub fn paint_air(&mut self, window: &mut Window, v: &View, _state: &GameState, renderer: &mut WorldRenderer, _time: f32) {
        let store = renderer.sprites.store.clone();
        self.aircraft.paint_air(window, v, &store, &mut self.bake);
    }
}
