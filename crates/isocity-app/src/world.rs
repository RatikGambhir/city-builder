//! The live game world: simulation session, camera and renderers.

use crate::iso::Camera;
use crate::render::world::{View, WorldRenderer};
use gpui::{App, Bounds, Pixels, Window};
use isocity_core::{GameState, LocalSession, Session};
use std::time::Instant;

pub struct World {
    pub session: LocalSession,
    pub renderer: WorldRenderer,
    pub camera: Camera,
    pub view_origin: (f32, f32),
    pub view_size: (f32, f32),
    centered: bool,
    last_frame: Instant,
    pub time: f32,
}

impl World {
    pub fn new(state: GameState) -> Self {
        Self {
            session: LocalSession::new(state),
            renderer: WorldRenderer::new(),
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
        }
    }

    pub fn state(&self) -> &GameState {
        self.session.state()
    }

    pub fn set_view(&mut self, bounds: Bounds<Pixels>) {
        self.view_origin = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
        self.view_size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        if !self.centered && self.view_size.0 > 10.0 {
            self.centered = true;
            if std::env::var_os("ISOCITY_CENTER").is_some() {
                let n = self.state().grid.size() as f32;
                self.camera.center_on(n / 2.0, n / 2.0, self.view_size.0, self.view_size.1);
            }
        }
    }

    pub fn frame(&mut self, window: &mut Window, _cx: &mut App) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.time += dt;
        self.session.advance(dt);
        self.renderer.scale_factor = window.scale_factor();
        self.renderer.sync(self.session.state());
        self.renderer.release_garbage(window);

        let v = View {
            origin_x: self.view_origin.0,
            origin_y: self.view_origin.1,
            width: self.view_size.0,
            height: self.view_size.1,
            cam: self.camera,
        };
        self.renderer.paint_background(window, &v);
        let size = self.session.state().grid.size();
        self.renderer.paint_ground(window, &v, size);
        let list = self.renderer.collect_buildings(self.session.state(), &v);
        for d in list {
            self.renderer
                .paint_building(window, &v, self.session.state(), d.x, d.y, self.time);
        }
    }
}
