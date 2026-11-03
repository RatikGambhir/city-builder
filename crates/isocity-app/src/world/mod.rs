//! The live game world: simulation session, camera, player interaction,
//! animated entities and per-frame painting of the map.

mod overlay;
mod input;
mod paint;

pub use overlay::OverlayMode;

use crate::entities::Entities;
use crate::iso::{Camera, TILE_H};
use crate::render::world::{View, WorldRenderer};
use crate::render::lighting::{DayNightMode, Lighting};
use gpui::{Bounds, Pixels};
use isocity_core::{Command, GameState, LocalSession, Rejection, Session, Tool};
use std::time::Instant;
use input::{Drag, PanCandidate};

#[derive(Default)]
pub struct Keys {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

pub struct Toast {
    pub text: String,
    pub until: f32,
}

pub struct World {
    pub session: LocalSession,
    pub renderer: WorldRenderer,
    pub entities: Entities,
    pub camera: Camera,
    pub view_origin: (f32, f32),
    pub view_size: (f32, f32),
    centered: bool,
    last_frame: Instant,
    pub time: f32,
    pub tool: Tool,
    pub overlay: OverlayMode,
    pub hovered: Option<(i32, i32)>,
    pub selected: Option<(i32, i32)>,
    drag: Option<Drag>,
    panning: Option<(f32, f32)>,
    pan_candidate: Option<PanCandidate>,
    pub keys: Keys,
    pub toast: Option<Toast>,
    pub fps: f32,
    pub perf: (f32, f32),
    pub lighting: Lighting,
    pub day_night: DayNightMode,
}

impl World {
    pub fn new(state: GameState) -> Self {
        let size = state.grid.size();
        Self {
            session: LocalSession::new(state),
            renderer: WorldRenderer::new(),
            entities: Entities::new(size),
            // Same starting view as the web version.
            camera: Camera {
                offset_x: 620.0,
                offset_y: 160.0,
                zoom: 1.0,
            },
            view_origin: (0.0, 0.0),
            view_size: (1.0, 1.0),
            centered: false,
            last_frame: Instant::now(),
            time: 0.0,
            tool: Tool::Select,
            overlay: OverlayMode::None,
            hovered: None,
            selected: None,
            drag: None,
            panning: None,
            pan_candidate: None,
            keys: Keys::default(),
            toast: None,
            fps: 60.0,
            perf: (0.0, 0.0),
            lighting: Lighting::default(),
            day_night: match std::env::var("ISOCITY_DAY_NIGHT").as_deref() {
                Ok("day") => DayNightMode::Day,
                Ok("night") => DayNightMode::Night,
                _ => DayNightMode::Auto,
            },
        }
    }

    pub fn state(&self) -> &GameState {
        self.session.state()
    }

    /// Replaces the city (load / new game) keeping renderers alive.
    pub fn replace_state(&mut self, state: GameState) {
        let size = state.grid.size();
        self.session = LocalSession::new(state);
        self.entities = Entities::new(size);
        self.selected = None;
        self.hovered = None;
        self.drag = None;
        self.camera = Camera {
            offset_x: 620.0,
            offset_y: 160.0,
            zoom: 1.0,
        };
        self.centered = false;
    }

    pub fn submit(&mut self, cmd: Command) -> bool {
        let old_size = self.state().grid.size();
        match self.session.submit(cmd) {
            Ok(()) => {
                let new_size = self.state().grid.size();
                if new_size != old_size {
                    self.grid_resized(old_size, new_size);
                }
                true
            }
            Err(Rejection::InsufficientFunds) => {
                self.show_toast("Not enough funds");
                false
            }
            Err(_) => false,
        }
    }

    pub fn show_toast(&mut self, text: &str) {
        self.toast = Some(Toast {
            text: text.to_string(),
            until: self.time + 2.5,
        });
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.drag = None;
        if tool != Tool::Select {
            self.selected = None;
        }
    }

    pub fn set_view(&mut self, bounds: Bounds<Pixels>) {
        self.view_origin = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
        self.view_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        if !self.centered && self.view_size.0 > 10.0 {
            self.centered = true;
            // Debug aid: `ISOCITY_CENTER=1` centers the map, `x,y[,zoom]` a tile.
            if let Ok(spec) = std::env::var("ISOCITY_CENTER") {
                let v: Vec<f32> = spec.split(',').filter_map(|p| p.trim().parse().ok()).collect();
                if v.len() >= 2 {
                    self.camera.zoom = v.get(2).copied().unwrap_or(1.0);
                    self.camera.center_on(v[0], v[1], self.view_size.0, self.view_size.1);
                } else {
                    self.center_on_map();
                }
            }
        }
    }

    /// Expanding or shrinking the city moves every tile diagonally by the
    /// border width; shift the camera by the same amount so the view stays
    /// on the same part of the city, and restart the visual entities whose
    /// tile coordinates are now stale.
    fn grid_resized(&mut self, old_size: i32, new_size: i32) {
        let border = (new_size - old_size) as f32 / 2.0;
        self.camera.offset_y -= border * TILE_H * self.camera.zoom;
        self.entities = Entities::new(new_size);
        self.selected = None;
        self.hovered = None;
        self.drag = None;
    }

    pub fn center_on_map(&mut self) {
        let n = self.state().grid.size() as f32;
        self.camera.center_on(n / 2.0, n / 2.0, self.view_size.0, self.view_size.1);
    }

    pub fn navigate_to(&mut self, gx: f32, gy: f32) {
        self.camera.center_on(gx, gy, self.view_size.0, self.view_size.1);
    }

    // ------------------------------------------------------------------
    // Frame
    // ------------------------------------------------------------------

    pub fn view(&self) -> View {
        View {
            origin_x: self.view_origin.0,
            origin_y: self.view_origin.1,
            width: self.view_size.0,
            height: self.view_size.1,
            cam: self.camera,
        }
    }

    /// Advances the simulation and animations. Called once per frame.
    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.time += dt;
        if dt > 0.0 {
            self.fps = self.fps * 0.95 + (1.0 / dt) * 0.05;
        }
        if std::env::var_os("ISOCITY_FPS").is_some() && (self.time % 2.0) < dt {
            eprintln!(
                "fps {:.1} update {:.2}ms paint {:.2}ms cars {} peds {} trains {} planes {}",
                self.fps,
                self.perf.0,
                self.perf.1,
                self.entities.traffic.cars.len(),
                self.entities.pedestrians.people.len(),
                self.entities.trains.trains.len(),
                self.entities.aircraft.planes.len()
            );
        }
        let pan = 520.0 * dt;
        if self.keys.up {
            self.camera.offset_y += pan;
        }
        if self.keys.down {
            self.camera.offset_y -= pan;
        }
        if self.keys.left {
            self.camera.offset_x += pan;
        }
        if self.keys.right {
            self.camera.offset_x -= pan;
        }
        let t0 = Instant::now();
        self.session.advance(dt);
        let v = self.view();
        self.entities.update(dt, self.session.state(), &v, &mut self.renderer);
        self.perf.0 = self.perf.0 * 0.9 + t0.elapsed().as_secs_f32() * 1000.0 * 0.1;
        if let Some(t) = &self.toast {
            if self.time > t.until {
                self.toast = None;
            }
        }
    }
}
