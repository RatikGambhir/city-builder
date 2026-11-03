//! Mouse and scroll input: placing, dragging, panning and zooming.

use crate::iso::world_to_grid;
use isocity_core::{sim::find_building_origin, BuildingType as B, Command, Tool, TrackType};
use std::collections::HashSet;
use super::World;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Axis {
    H,
    V,
}

pub(super) struct Drag {
    pub(super) start: (i32, i32),
    pub(super) end: (i32, i32),
    axis: Option<Axis>,
    placed: HashSet<(i32, i32)>,
}

pub(super) struct PanCandidate {
    pub(super) start: (f32, f32),
    tile: (i32, i32),
}

impl World {
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

    pub(super) fn place(&mut self, x: i32, y: i32) {
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

    pub(super) fn tool_shows_drag_grid(&self) -> bool {
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
}
