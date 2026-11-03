//! Immediate-mode vector helpers on top of GPUI's paint API, in screen space.

use gpui::{point, px, Bounds, Corners, Hsla, PathBuilder, Pixels, Point, Window};

pub fn rgba(hex: u32, a: f32) -> Hsla {
    let c = gpui::rgb(hex);
    let mut h: Hsla = c.into();
    h.a = a;
    h
}

pub fn p(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

pub fn fill_poly(window: &mut Window, pts: &[(f32, f32)], color: Hsla) {
    if pts.len() < 3 {
        return;
    }
    let mut b = PathBuilder::fill();
    b.move_to(p(pts[0].0, pts[0].1));
    for q in &pts[1..] {
        b.line_to(p(q.0, q.1));
    }
    b.close();
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

pub fn stroke_poly(window: &mut Window, pts: &[(f32, f32)], close: bool, width: f32, color: Hsla) {
    if pts.len() < 2 {
        return;
    }
    let mut b = PathBuilder::stroke(px(width));
    b.move_to(p(pts[0].0, pts[0].1));
    for q in &pts[1..] {
        b.line_to(p(q.0, q.1));
    }
    if close {
        b.close();
    }
    if let Ok(path) = b.build() {
        window.paint_path(path, color);
    }
}

pub fn line(window: &mut Window, a: (f32, f32), b: (f32, f32), width: f32, color: Hsla) {
    stroke_poly(window, &[a, b], false, width, color);
}

/// Axis-aligned rectangle (cheap: GPU quad).
pub fn rect(window: &mut Window, x: f32, y: f32, w: f32, h: f32, color: Hsla) {
    window.paint_quad(gpui::fill(Bounds::new(p(x, y), gpui::size(px(w), px(h))), color));
}

/// Ellipse approximated by a fully rounded quad.
pub fn ellipse(window: &mut Window, cx: f32, cy: f32, rx: f32, ry: f32, color: Hsla) {
    let r = rx.min(ry);
    window.paint_quad(
        gpui::fill(Bounds::new(p(cx - rx, cy - ry), gpui::size(px(rx * 2.0), px(ry * 2.0))), color)
            .corner_radii(Corners::all(px(r))),
    );
}

pub fn circle(window: &mut Window, cx: f32, cy: f32, r: f32, color: Hsla) {
    ellipse(window, cx, cy, r, r, color);
}

/// Diamond (tile) outline points for a tile whose bounding box starts at `(x, y)`.
pub fn diamond(x: f32, y: f32, w: f32, h: f32) -> [(f32, f32); 4] {
    [(x + w / 2.0, y), (x + w, y + h / 2.0), (x + w / 2.0, y + h), (x, y + h / 2.0)]
}
