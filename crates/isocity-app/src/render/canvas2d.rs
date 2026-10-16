//! A tiny Canvas2D-like wrapper around `tiny_skia` so drawing code can be
//! ported from the web version almost line by line.

use tiny_skia::{
    FillRule, LineCap, Paint, PathBuilder, Pattern, Pixmap, PixmapRef, Rect, SpreadMode,
    Stroke, StrokeDash, Transform,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    pub const fn hex(v: u32) -> Self {
        Rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, 255)
    }

    pub const fn hexa(v: u32, a: f32) -> Self {
        Rgba((v >> 16) as u8, (v >> 8) as u8, v as u8, (a * 255.0) as u8)
    }

    pub fn alpha(self, a: f32) -> Self {
        Rgba(self.0, self.1, self.2, (self.3 as f32 * a) as u8)
    }

    fn paint(self) -> Paint<'static> {
        let mut p = Paint::default();
        p.set_color_rgba8(self.0, self.1, self.2, self.3);
        p.anti_alias = true;
        p
    }
}

pub type P = (f32, f32);

pub struct Canvas {
    pub pixmap: Pixmap,
    pub transform: Transform,
}

impl Canvas {
    pub fn new(w: u32, h: u32, transform: Transform) -> Option<Self> {
        Some(Self {
            pixmap: Pixmap::new(w.max(1), h.max(1))?,
            transform,
        })
    }

    fn poly_path(points: &[P], close: bool) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        let (first, rest) = points.split_first()?;
        pb.move_to(first.0, first.1);
        for p in rest {
            pb.line_to(p.0, p.1);
        }
        if close {
            pb.close();
        }
        pb.finish()
    }

    pub fn fill_poly(&mut self, points: &[P], color: Rgba) {
        if let Some(path) = Self::poly_path(points, true) {
            self.pixmap
                .fill_path(&path, &color.paint(), FillRule::Winding, self.transform, None);
        }
    }

    pub fn fill_path(&mut self, path: &tiny_skia::Path, color: Rgba) {
        self.pixmap
            .fill_path(path, &color.paint(), FillRule::Winding, self.transform, None);
    }

    pub fn stroke_poly(&mut self, points: &[P], close: bool, width: f32, color: Rgba, dash: Option<&[f32]>) {
        if let Some(path) = Self::poly_path(points, close) {
            self.stroke_path(&path, width, color, dash, LineCap::Butt);
        }
    }

    pub fn stroke_path(&mut self, path: &tiny_skia::Path, width: f32, color: Rgba, dash: Option<&[f32]>, cap: LineCap) {
        let mut stroke = Stroke {
            width,
            line_cap: cap,
            ..Stroke::default()
        };
        if let Some(d) = dash {
            stroke.dash = StrokeDash::new(d.to_vec(), 0.0);
        }
        self.pixmap
            .stroke_path(path, &color.paint(), &stroke, self.transform, None);
    }

    pub fn line(&mut self, a: P, b: P, width: f32, color: Rgba) {
        self.stroke_poly(&[a, b], false, width, color, None);
    }

    pub fn line_round(&mut self, a: P, b: P, width: f32, color: Rgba) {
        if let Some(path) = Self::poly_path(&[a, b], false) {
            self.stroke_path(&path, width, color, None, LineCap::Round);
        }
    }

    pub fn dashed_line(&mut self, a: P, b: P, width: f32, color: Rgba, dash: &[f32], cap: LineCap) {
        if let Some(path) = Self::poly_path(&[a, b], false) {
            self.stroke_path(&path, width, color, Some(dash), cap);
        }
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Rgba) {
        if let Some(r) = Rect::from_xywh(x, y, w, h) {
            self.pixmap.fill_rect(r, &color.paint(), self.transform, None);
        }
    }

    pub fn fill_circle(&mut self, cx: f32, cy: f32, r: f32, color: Rgba) {
        if let Some(path) = PathBuilder::from_circle(cx, cy, r) {
            self.fill_path(&path, color);
        }
    }

    pub fn fill_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, color: Rgba) {
        if let Some(r) = Rect::from_xywh(cx - rx, cy - ry, rx * 2.0, ry * 2.0) {
            if let Some(path) = PathBuilder::from_oval(r) {
                self.fill_path(&path, color);
            }
        }
    }

    /// Fills `clip` with the `src` rectangle of `image` stretched to `dst`.
    pub fn fill_poly_with_image(
        &mut self,
        clip: &[P],
        image: PixmapRef,
        src: (f32, f32, f32, f32),
        dst: (f32, f32, f32, f32),
        opacity: f32,
    ) {
        let Some(path) = Self::poly_path(clip, true) else {
            return;
        };
        let (sx, sy, sw, sh) = src;
        let (dx, dy, dw, dh) = dst;
        let pattern_t = Transform::from_translate(dx, dy)
            .pre_scale(dw / sw, dh / sh)
            .pre_translate(-sx, -sy);
        let shader = Pattern::new(
            image,
            SpreadMode::Pad,
            tiny_skia::FilterQuality::Bilinear,
            opacity,
            pattern_t,
        );
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        self.pixmap
            .fill_path(&path, &paint, FillRule::Winding, self.transform, None);
    }

    /// Quadratic curve strip helper used by rails.
    pub fn quad_path(a: P, c: P, b: P) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        pb.move_to(a.0, a.1);
        pb.quad_to(c.0, c.1, b.0, b.1);
        pb.finish()
    }

    /// Straight-alpha BGRA bytes, the layout GPUI images expect.
    pub fn into_bgra(self) -> (u32, u32, Vec<u8>) {
        let (w, h) = (self.pixmap.width(), self.pixmap.height());
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for p in self.pixmap.pixels() {
            let c = p.demultiply();
            out.extend_from_slice(&[c.blue(), c.green(), c.red(), c.alpha()]);
        }
        (w, h, out)
    }
}
