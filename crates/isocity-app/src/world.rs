//! The live game world: simulation session, camera, player interaction,
//! animated entities and per-frame painting of the map.

use crate::entities::Entities;
use crate::iso::{grid_to_world, tile_center, world_to_grid, Camera, TILE_H, TILE_W};
use crate::render::shapes::{self, rgba};
use crate::render::world::{View, WorldRenderer};
use crate::render::lighting::{DayNightMode, Lighting};
use crate::theme;
use gpui::{px, App, Bounds, Hsla, Pixels, SharedString, TextRun, Window};
use isocity_core::{
    sim::{effective_service_range, find_building_origin},
    BuildingType as B, Command, GameState, LocalSession, Rejection, Session, Tool, TrackType,
};
use std::collections::HashSet;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OverlayMode {
    None,
    Power,
    Water,
    Fire,
    Police,
    Health,
    Education,
    Subway,
}

impl OverlayMode {
    pub const ALL: [OverlayMode; 8] = [
        OverlayMode::None,
        OverlayMode::Power,
        OverlayMode::Water,
        OverlayMode::Fire,
        OverlayMode::Police,
        OverlayMode::Health,
        OverlayMode::Education,
        OverlayMode::Subway,
    ];

    pub fn icon(self) -> &'static str {
        match self {
            OverlayMode::None => "close",
            OverlayMode::Power => "power",
            OverlayMode::Water => "water",
            OverlayMode::Fire => "fire",
            OverlayMode::Police => "safety",
            OverlayMode::Health => "health",
            OverlayMode::Education => "education",
            OverlayMode::Subway => "subway",
        }
    }

    pub fn color(self) -> u32 {
        match self {
            OverlayMode::None => 0x60a5fa,
            OverlayMode::Power => 0xf59e0b,
            OverlayMode::Water => 0x3b82f6,
            OverlayMode::Fire => 0xef4444,
            OverlayMode::Police => 0x2563eb,
            OverlayMode::Health => 0x22c55e,
            OverlayMode::Education => 0xa855f7,
            OverlayMode::Subway => 0xeab308,
        }
    }

    fn building_types(self) -> &'static [B] {
        match self {
            OverlayMode::Power => &[B::PowerPlant],
            OverlayMode::Water => &[B::WaterTower],
            OverlayMode::Fire => &[B::FireStation],
            OverlayMode::Police => &[B::PoliceStation],
            OverlayMode::Health => &[B::Hospital],
            OverlayMode::Education => &[B::School, B::University],
            _ => &[],
        }
    }

    /// Overlay automatically shown while a related tool is selected.
    pub fn for_tool(tool: Tool) -> Option<OverlayMode> {
        Some(match tool {
            Tool::PowerPlant => OverlayMode::Power,
            Tool::WaterTower => OverlayMode::Water,
            Tool::FireStation => OverlayMode::Fire,
            Tool::PoliceStation => OverlayMode::Police,
            Tool::Hospital => OverlayMode::Health,
            Tool::School | Tool::University => OverlayMode::Education,
            Tool::SubwayStation | Tool::Subway => OverlayMode::Subway,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Axis {
    H,
    V,
}

struct Drag {
    start: (i32, i32),
    end: (i32, i32),
    axis: Option<Axis>,
    placed: HashSet<(i32, i32)>,
}

struct PanCandidate {
    start: (f32, f32),
    tile: (i32, i32),
}

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

    fn local(&self, pos: (f32, f32)) -> (f32, f32) {
        (pos.0 - self.view_origin.0, pos.1 - self.view_origin.1)
    }

    fn tile_at(&self, pos: (f32, f32)) -> (i32, i32) {
        let (lx, ly) = self.local(pos);
        let (wx, wy) = self.camera.screen_to_world(lx, ly);
        world_to_grid(wx, wy)
    }

    fn in_grid(&self, t: (i32, i32)) -> bool {
        self.state().grid.in_bounds(t.0, t.1)
    }

    fn place(&mut self, x: i32, y: i32) {
        let tool = self.tool;
        self.submit(Command::ApplyTool { tool, x, y });
    }

    fn select_tile(&mut self, t: (i32, i32)) {
        let origin = find_building_origin(&self.state().grid, t.0, t.1);
        self.selected = Some(origin.map(|(x, y, _)| (x, y)).unwrap_or(t));
    }

    // ------------------------------------------------------------------
    // Input
    // ------------------------------------------------------------------

    pub fn mouse_down(&mut self, pos: (f32, f32), button: gpui::MouseButton, alt: bool) {
        if button != gpui::MouseButton::Left || alt {
            self.panning = Some(pos);
            return;
        }
        let t = self.tile_at(pos);
        if !self.in_grid(t) {
            self.panning = Some(pos);
            return;
        }
        if self.tool == Tool::Select {
            let k = self.state().grid.at(t.0, t.1).building.kind;
            if matches!(k, B::Empty | B::Grass | B::Water) {
                self.pan_candidate = Some(PanCandidate { start: pos, tile: t });
            } else {
                self.select_tile(t);
            }
            return;
        }
        let mut placed = HashSet::new();
        placed.insert(t);
        self.drag = Some(Drag {
            start: t,
            end: t,
            axis: None,
            placed,
        });
        if !self.tool_shows_drag_grid() {
            self.place(t.0, t.1);
        }
    }

    fn tool_shows_drag_grid(&self) -> bool {
        matches!(
            self.tool,
            Tool::ZoneResidential | Tool::ZoneCommercial | Tool::ZoneIndustrial | Tool::ZoneDezone
        )
    }

    pub fn mouse_move(&mut self, pos: (f32, f32), left_down: bool) {
        if let Some(last) = self.panning {
            self.camera.offset_x += pos.0 - last.0;
            self.camera.offset_y += pos.1 - last.1;
            self.panning = Some(pos);
            return;
        }
        if let Some(pc) = &self.pan_candidate {
            if left_down {
                let d = ((pos.0 - pc.start.0).powi(2) + (pos.1 - pc.start.1).powi(2)).sqrt();
                if d > 6.0 {
                    let start = pc.start;
                    self.pan_candidate = None;
                    self.camera.offset_x += pos.0 - start.0;
                    self.camera.offset_y += pos.1 - start.1;
                    self.panning = Some(pos);
                    return;
                }
            } else {
                self.pan_candidate = None;
            }
        }
        let t = self.tile_at(pos);
        self.hovered = if self.in_grid(t) { Some(t) } else { None };
        if !left_down {
            if self.drag.is_some() {
                self.finish_drag();
            }
            return;
        }
        let tool = self.tool;
        let shows_grid = self.tool_shows_drag_grid();
        if self.drag.is_none() || !self.state().grid.in_bounds(t.0, t.1) {
            return;
        }
        if shows_grid {
            if let Some(d) = self.drag.as_mut() {
                d.end = t;
            }
        } else if tool.is_line_tool() {
            let mut todo = Vec::new();
            if let Some(drag) = self.drag.as_mut() {
                let dx = (t.0 - drag.start.0).abs();
                let dy = (t.1 - drag.start.1).abs();
                if drag.axis.is_none() && (dx > 0 || dy > 0) {
                    drag.axis = Some(if dx >= dy { Axis::H } else { Axis::V });
                }
                let target = match drag.axis {
                    Some(Axis::H) => (t.0, drag.start.1),
                    Some(Axis::V) => (drag.start.0, t.1),
                    None => t,
                };
                drag.end = target;
                let (x0, x1) = (drag.start.0.min(target.0), drag.start.0.max(target.0));
                let (y0, y1) = (drag.start.1.min(target.1), drag.start.1.max(target.1));
                for x in x0..=x1 {
                    for y in y0..=y1 {
                        if drag.placed.insert((x, y)) {
                            todo.push((x, y));
                        }
                    }
                }
            }
            for (x, y) in todo {
                if !self.state().grid.is_water(x, y) {
                    self.place(x, y);
                }
            }
        } else if self.drag.as_ref().is_some_and(|d| d.end != t) {
            if let Some(d) = self.drag.as_mut() {
                d.end = t;
            }
            self.place(t.0, t.1);
        }
    }

    pub fn mouse_up(&mut self, _pos: (f32, f32)) {
        self.panning = None;
        if let Some(pc) = self.pan_candidate.take() {
            if self.tool == Tool::Select {
                self.select_tile(pc.tile);
            }
        }
        self.finish_drag();
    }

    fn finish_drag(&mut self) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let (x0, x1) = (drag.start.0.min(drag.end.0), drag.start.0.max(drag.end.0));
        let (y0, y1) = (drag.start.1.min(drag.end.1), drag.start.1.max(drag.end.1));
        if self.tool_shows_drag_grid() {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    self.place(x, y);
                }
            }
        }
        if matches!(self.tool, Tool::Road | Tool::Rail) {
            let mut path = Vec::new();
            for x in x0..=x1 {
                for y in y0..=y1 {
                    path.push((x, y));
                }
            }
            let track = if self.tool == Tool::Road { TrackType::Road } else { TrackType::Rail };
            self.submit(Command::FinishTrackDrag { path, track });
        }
    }

    pub fn scroll(&mut self, pos: (f32, f32), delta_y: f32) {
        let (ax, ay) = self.local(pos);
        let factor = (1.0 - delta_y * 0.0015).clamp(0.6, 1.6);
        let z = self.camera.zoom * factor;
        self.camera.zoom_at(z, ax, ay);
    }

    pub fn zoom_by(&mut self, factor: f32) {
        let (ax, ay) = (self.view_size.0 / 2.0, self.view_size.1 / 2.0);
        let z = self.camera.zoom * factor;
        self.camera.zoom_at(z, ax, ay);
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

    pub fn paint(&mut self, window: &mut Window, cx: &mut App) {
        let t0 = Instant::now();
        self.paint_inner(window, cx);
        self.perf.1 = self.perf.1 * 0.9 + t0.elapsed().as_secs_f32() * 1000.0 * 0.1;
    }

    fn paint_inner(&mut self, window: &mut Window, cx: &mut App) {
        self.renderer.scale_factor = window.scale_factor();
        self.renderer.sync(self.session.state());
        self.renderer.release_garbage(window);
        let v = self.view();
        let state = self.session.state();

        self.renderer.paint_background(window, &v);
        self.renderer.paint_ground(window, &v, state.grid.size());
        self.paint_drag_preview(window, &v);
        self.entities.paint_ground_level(window, &v, state, &mut self.renderer, self.time);
        self.paint_traffic_lights(window, &v);
        self.entities.crossings.paint(window, &v);

        let list = self.renderer.collect_buildings(state, &v);
        for d in list {
            self.renderer.paint_building(window, &v, state, d.x, d.y, self.time);
        }
        self.entities.paint_above_buildings(window, &v, state, &mut self.renderer, self.time);
        self.paint_overlay(window, &v);
        self.paint_highlights(window, &v);
        self.entities.paint_air(window, &v, state, &mut self.renderer, self.time);
        self.lighting.release_garbage(window);
        let hour = self.day_night.visual_hour(state.hour);
        self.lighting.paint(window, &v, state, hour);
        self.paint_labels(window, cx, &v);
    }

    fn diamond_screen(v: &View, x: i32, y: i32) -> [(f32, f32); 4] {
        let (wx, wy) = grid_to_world(x as f32, y as f32);
        let d = shapes::diamond(wx, wy, TILE_W, TILE_H);
        d.map(|(a, b)| v.to_screen(a, b))
    }

    fn paint_drag_preview(&self, window: &mut Window, v: &View) {
        let Some(drag) = &self.drag else {
            return;
        };
        let (x0, x1) = (drag.start.0.min(drag.end.0), drag.start.0.max(drag.end.0));
        let (y0, y1) = (drag.start.1.min(drag.end.1), drag.start.1.max(drag.end.1));
        let grid_mode = self.tool_shows_drag_grid();
        if !grid_mode && !self.tool.is_line_tool() {
            return;
        }
        let color = match self.tool {
            Tool::ZoneResidential => 0x22c55e,
            Tool::ZoneCommercial => 0x3b82f6,
            Tool::ZoneIndustrial => 0xf59e0b,
            Tool::ZoneDezone => 0xef4444,
            _ => 0xffffff,
        };
        for x in x0..=x1 {
            for y in y0..=y1 {
                if !grid_mode && !self.state().grid.is_water(x, y) {
                    continue;
                }
                let d = Self::diamond_screen(v, x, y);
                shapes::fill_poly(window, &d, rgba(color, 0.25));
                shapes::stroke_poly(window, &d, true, 1.0, rgba(color, 0.9));
            }
        }
    }

    fn paint_traffic_lights(&self, window: &mut Window, v: &View) {
        if v.cam.zoom < 0.6 {
            return;
        }
        let Some(map) = self.renderer.tile_map.as_ref() else {
            return;
        };
        let state = crate::entities::traffic::light_state(self.entities.traffic_time);
        let (vx, vy, vw, vh) = v.world_rect();
        let z = v.cam.zoom;
        let s = if z > 0.8 { 1.0 } else { 0.8 };
        for t in self.session.state().grid.iter() {
            if t.building.kind != B::Road || !map.is_intersection(t.x, t.y) {
                continue;
            }
            let (x, y) = grid_to_world(t.x as f32, t.y as f32);
            if x + TILE_W < vx || x > vx + vw || y + TILE_H < vy || y - 20.0 > vy + vh {
                continue;
            }
            let (n, e, so, w) = (
                map.has_road(t.x - 1, t.y),
                map.has_road(t.x, t.y - 1),
                map.has_road(t.x + 1, t.y),
                map.has_road(t.x, t.y + 1),
            );
            let mut corners = Vec::new();
            if n && w {
                corners.push((0.08, 0.42, true));
            }
            if n && e {
                corners.push((0.5, 0.08, false));
            }
            if so && w {
                corners.push((0.5, 0.92, false));
            }
            if so && e {
                corners.push((0.92, 0.58, true));
            }
            for (ox, oy, ns) in corners {
                let (lx, ly) = v.to_screen(x + TILE_W * ox, y + TILE_H * oy);
                let pole = 6.0 * s * z;
                let ls = 1.5 * s * z;
                shapes::line(window, (lx, ly), (lx, ly - pole), 0.8 * z, rgba(0x374151, 1.0));
                let (hw, hh) = (ls * 2.0, ls * 4.0);
                shapes::rect(window, lx - hw / 2.0, ly - pole - hh, hw, hh, rgba(0x1f2937, 1.0));
                use crate::entities::traffic::LightState::*;
                let (red, yellow, green) = match (ns, state) {
                    (true, GreenNs) | (false, GreenEw) => (false, false, true),
                    (true, YellowNs) | (false, YellowEw) => (false, true, false),
                    _ => (true, false, false),
                };
                let base_y = ly - pole - hh + ls * 0.8;
                let r = ls * 0.6;
                let off = 0x1f2937;
                shapes::circle(window, lx, base_y, r, rgba(if red { 0xef4444 } else { off }, 1.0));
                shapes::circle(window, lx, base_y + ls * 1.2, r, rgba(if yellow { 0xfbbf24 } else { off }, 1.0));
                shapes::circle(window, lx, base_y + ls * 2.4, r, rgba(if green { 0x22c55e } else { off }, 1.0));
            }
        }
    }

    fn paint_overlay(&self, window: &mut Window, v: &View) {
        let mode = if self.overlay == OverlayMode::None {
            OverlayMode::for_tool(self.tool).unwrap_or(OverlayMode::None)
        } else {
            self.overlay
        };
        if mode == OverlayMode::None {
            return;
        }
        let state = self.session.state();
        let s = &state.services;
        let (vx, vy, vw, vh) = v.world_rect();
        for t in state.grid.iter() {
            let (wx, wy) = grid_to_world(t.x as f32, t.y as f32);
            if wx + TILE_W < vx || wx > vx + vw || wy + TILE_H < vy || wy > vy + vh {
                continue;
            }
            let k = t.building.kind;
            let needs = !matches!(k, B::Empty | B::Grass | B::Water | B::Road | B::Rail | B::Tree);
            if s.police.is_empty() {
                continue;
            }
            let i = s.idx(t.x, t.y);
            let fill: Option<Hsla> = match mode {
                OverlayMode::Subway => {
                    if k == B::Water {
                        None
                    } else if t.has_subway {
                        Some(rgba(0xf59e0b, 0.7))
                    } else {
                        Some(rgba(0x281e14, 0.4))
                    }
                }
                _ if !needs || k == B::Bridge => None,
                OverlayMode::Power => (!t.building.powered).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Water => (!t.building.watered).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Fire => (s.fire[i] <= 0.0).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Police => (s.police[i] <= 0.0).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Health => (s.health[i] <= 0.0).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Education => (s.education[i] <= 0.0).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::None => None,
            };
            if let Some(c) = fill {
                shapes::fill_poly(window, &Self::diamond_screen(v, t.x, t.y), c);
            }
        }
        let color = mode.color();
        for t in state.grid.iter() {
            if !mode.building_types().contains(&t.building.kind) || t.building.under_construction() || t.building.abandoned {
                continue;
            }
            let Some(range) = effective_service_range(t.building.kind, t.building.level) else {
                continue;
            };
            let (cx, cy) = tile_center(t.x as f32, t.y as f32);
            let (sx, sy) = v.to_screen(cx, cy);
            let z = v.cam.zoom;
            let rx = range as f32 * TILE_W / 2.0 * z;
            let ry = range as f32 * TILE_H / 2.0 * z;
            let pts: Vec<(f32, f32)> = (0..64)
                .map(|i| {
                    let a = i as f32 / 64.0 * std::f32::consts::TAU;
                    (sx + rx * a.cos(), sy + ry * a.sin())
                })
                .collect();
            shapes::fill_poly(window, &pts, rgba(color, 0.12));
            shapes::stroke_poly(window, &pts, true, 2.0, rgba(color, 0.8));
            shapes::stroke_poly(window, &Self::diamond_screen(v, t.x, t.y), true, 3.0, rgba(color, 1.0));
            shapes::circle(window, sx, sy, 4.0, rgba(color, 1.0));
        }
    }

    fn paint_highlights(&self, window: &mut Window, v: &View) {
        if let Some((x, y)) = self.selected {
            let fp = self.state().grid.get(x, y).map(|t| t.building.kind.size()).unwrap_or(isocity_core::Footprint { width: 1, height: 1 });
            for dx in 0..fp.width {
                for dy in 0..fp.height {
                    let d = Self::diamond_screen(v, x + dx, y + dy);
                    shapes::fill_poly(window, &d, rgba(0x60a5fa, 0.25));
                    shapes::stroke_poly(window, &d, true, 2.0, rgba(0x60a5fa, 1.0));
                }
            }
        }
        if self.drag.is_some() && self.tool_shows_drag_grid() {
            return;
        }
        let Some((hx, hy)) = self.hovered else {
            return;
        };
        let size = if self.tool == Tool::Select { 1 } else { self.tool.size() };
        let valid = self.placement_valid(hx, hy);
        let (fill, stroke) = if valid {
            (rgba(0xffffff, 0.25), rgba(0xffffff, 1.0))
        } else {
            (rgba(0xef4444, 0.3), rgba(0xef4444, 1.0))
        };
        for dx in 0..size {
            for dy in 0..size {
                let d = Self::diamond_screen(v, hx + dx, hy + dy);
                shapes::fill_poly(window, &d, fill);
                shapes::stroke_poly(window, &d, true, 2.0, stroke);
            }
        }
    }

    /// Whether the current building tool could be placed at the hovered tile.
    fn placement_valid(&self, x: i32, y: i32) -> bool {
        let Some(kind) = self.tool.building() else {
            return true;
        };
        if matches!(kind, B::Road | B::Rail | B::Tree) {
            return true;
        }
        let grid = &self.state().grid;
        let fp = kind.size();
        for dy in 0..fp.height {
            for dx in 0..fp.width {
                match grid.kind(x + dx, y + dy) {
                    Some(B::Grass) | Some(B::Tree) => {}
                    _ => return false,
                }
            }
        }
        if kind.requires_water_adjacency() {
            return isocity_core::place::water_adjacency(grid, x, y, fp.width, fp.height).0;
        }
        self.state().stats.money >= self.tool.cost()
    }

    fn paint_labels(&self, window: &mut Window, cx: &mut App, v: &View) {
        let state = self.session.state();
        let size = (12.0 / v.cam.zoom).max(10.0) * v.cam.zoom;
        let font = gpui::font(theme::FONT);
        for wb in &state.water_bodies {
            if wb.tiles.is_empty() {
                continue;
            }
            let (cx_w, cy_w) = tile_center(wb.center_x, wb.center_y);
            let (sx, sy) = v.to_screen(cx_w, cy_w);
            if sx < v.origin_x - 100.0 || sx > v.origin_x + v.width + 100.0 || sy < v.origin_y - 50.0 || sy > v.origin_y + v.height + 50.0 {
                continue;
            }
            let text: SharedString = wb.name.clone().into();
            let run = |color: Hsla| TextRun {
                len: text.len(),
                font: font.clone(),
                color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shadow = window.text_system().shape_line(text.clone(), px(size), &[run(rgba(0x000000, 0.5))], None);
            let line = window.text_system().shape_line(text.clone(), px(size), &[run(rgba(0xffffff, 0.9))], None);
            let w = f32::from(line.width);
            let origin = gpui::point(px(sx - w / 2.0), px(sy - size * 0.6));
            for (ox, oy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                let _ = shadow.paint(gpui::point(origin.x + px(ox), origin.y + px(oy)), px(size * 1.2), window, cx);
            }
            let _ = line.paint(origin, px(size * 1.2), window, cx);
        }
    }
}
