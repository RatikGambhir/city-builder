//! The minimap: painting it and click-to-navigate.

use crate::ui::widgets::self;
use crate::theme;
use crate::world::World;
use gpui::{canvas, div, prelude::*, px, Bounds, Context, MouseButton, MouseDownEvent, Pixels, Point, Window};
use super::GameView;

impl GameView {
    pub(super) fn minimap_navigate(&mut self, pos: Point<Pixels>) {
        let Some(b) = *self.minimap_bounds.borrow() else {
            return;
        };
        let mut w = self.world.borrow_mut();
        let n = w.state().grid.size() as f32;
        let gx = f32::from(pos.x - b.origin.x) / f32::from(b.size.width) * n;
        let gy = f32::from(pos.y - b.origin.y) / f32::from(b.size.height) * n;
        w.navigate_to(gx - 0.5, gy - 0.5);
    }

    pub(super) fn render_minimap(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let world = self.world.clone();
        let bounds_store = self.minimap_bounds.clone();
        let legend = |color: u32, l: &str| {
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(div().size(px(8.0)).rounded(px(2.0)).bg(gpui::rgb(color)))
                .child(div().text_color(theme::muted_foreground()).child(l.to_string()))
        };
        widgets::card()
            .id("minimap")
            .occlude()
            .absolute()
            .bottom(px(24.0))
            .right(px(32.0))
            .p(px(12.0))
            .child(div().mb(px(8.0)).child(widgets::section_label("MINIMAP")))
            .child(
                div()
                    .size(px(140.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(theme::alpha(theme::border(), 0.6))
                    .overflow_hidden()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, e: &MouseDownEvent, _, cx| {
                            this.minimap_drag = true;
                            this.minimap_navigate(e.position);
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .child(
                        canvas(
                            move |b, _, _| {
                                *bounds_store.borrow_mut() = Some(b);
                            },
                            move |b, _, window, _cx| {
                                paint_minimap(&world.borrow(), b, window);
                            },
                        )
                        .size_full(),
                    ),
            )
            .child(
                div()
                    .mt(px(8.0))
                    .flex()
                    .gap(px(8.0))
                    .text_size(px(9.0))
                    .child(legend(0x22c55e, "R"))
                    .child(legend(0x3b82f6, "C"))
                    .child(legend(0xf59e0b, "I"))
                    .child(legend(0xec4899, "S")),
            )
    }
}

fn minimap_color(t: &isocity_core::Tile) -> u32 {
    use isocity_core::{BuildingType as B, ZoneType as Z};
    let k = t.building.kind;
    let built = k != B::Grass;
    if k == B::Water {
        0x0ea5e9
    } else if matches!(k, B::Road | B::Bridge) {
        0x6b7280
    } else if k == B::Rail {
        0x78716c
    } else if k == B::Tree {
        0x166534
    } else if t.building.on_fire {
        0xef4444
    } else if t.zone == Z::Residential {
        if built { 0x22c55e } else { 0x14532d }
    } else if t.zone == Z::Commercial {
        if built { 0x38bdf8 } else { 0x1d4ed8 }
    } else if t.zone == Z::Industrial {
        if built { 0xf59e0b } else { 0xb45309 }
    } else if matches!(k, B::PoliceStation | B::FireStation | B::Hospital | B::School | B::University) {
        0xc084fc
    } else if k == B::PowerPlant {
        0xf97316
    } else if k == B::WaterTower {
        0x06b6d4
    } else if k.is_park() {
        0x84cc16
    } else if !k.is_terrain() || k == B::Empty {
        0xec4899
    } else {
        0x2d5a3d
    }
}

fn paint_minimap(w: &World, b: Bounds<Pixels>, window: &mut Window) {
    use crate::render::shapes;
    let st = w.state();
    let n = st.grid.size();
    let (ox, oy) = (f32::from(b.origin.x), f32::from(b.origin.y));
    let size = f32::from(b.size.width);
    window.paint_quad(gpui::fill(b, gpui::rgb(0x0b1723)));
    let cell = size / n as f32;
    // Merge horizontal runs of equal color into single quads.
    for y in 0..n {
        let mut run_start = 0;
        let mut run_color = minimap_color(st.grid.at(0, y));
        for x in 1..=n {
            let c = if x < n { minimap_color(st.grid.at(x, y)) } else { u32::MAX };
            if c != run_color {
                shapes::rect(
                    window,
                    ox + run_start as f32 * cell,
                    oy + y as f32 * cell,
                    (x - run_start) as f32 * cell + 0.3,
                    cell + 0.3,
                    shapes::rgba(run_color, 1.0),
                );
                run_start = x;
                run_color = c;
            }
        }
    }
    // Viewport: project the four screen corners into grid space.
    let v = w.view();
    let (vx, vy, vw, vh) = v.world_rect();
    let corners = [(vx, vy), (vx + vw, vy), (vx + vw, vy + vh), (vx, vy + vh)];
    let pts: Vec<(f32, f32)> = corners
        .iter()
        .map(|&(wx, wy)| {
            let (gx, gy) = crate::iso::world_to_grid_f(wx, wy);
            (ox + (gx + 0.5) * cell, oy + (gy + 0.5) * cell)
        })
        .collect();
    window.with_content_mask(Some(gpui::ContentMask { bounds: b }), |window| {
        shapes::stroke_poly(window, &pts, true, 1.5, shapes::rgba(0xffffff, 0.9));
    });
}
