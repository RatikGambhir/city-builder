//! Per-frame painting of the map and its interactive layers.

use crate::iso::{grid_to_world, tile_center, TILE_H, TILE_W};
use crate::render::shapes::{self, rgba};
use crate::render::world::View;
use crate::theme;
use gpui::{px, App, Hsla, SharedString, TextRun, Window};
use isocity_core::{BuildingType as B, Session, Tool};
use std::time::Instant;
use super::World;

impl World {
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

    pub(super) fn diamond_screen(v: &View, x: i32, y: i32) -> [(f32, f32); 4] {
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
