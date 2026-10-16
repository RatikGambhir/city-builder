//! Day/night lighting, ported from the web version's `lightingSystem.ts`.
//!
//! The web version draws a dark ambient layer, punches light holes into it
//! with `destination-out`, adds coloured glows and finally composites the
//! layer with `mix-blend-mode: multiply`. GPUI has neither custom blend
//! modes nor offscreen canvases, so the layer is rendered with tiny-skia at
//! reduced resolution and converted to an equivalent alpha-blended overlay.

use super::world::View;
use crate::iso::{grid_to_world, TILE_H, TILE_W};
use gpui::{Corners, RenderImage, Window};
use image::{Frame, ImageBuffer};
use isocity_core::{BuildingType as B, GameState};
use smallvec::SmallVec;
use std::sync::Arc;
use tiny_skia::{
    BlendMode, Color, FillRule, GradientStop, Paint, PathBuilder, Pixmap, Point, RadialGradient,
    SpreadMode, Transform,
};

/// Lighting layer resolution relative to logical pixels. The layer is made
/// of soft gradients only, so it upscales without visible artifacts.
const LAYER_SCALE: f32 = 0.5;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DayNightMode {
    #[default]
    Auto,
    Day,
    Night,
}

impl DayNightMode {
    pub const ALL: [DayNightMode; 3] = [DayNightMode::Auto, DayNightMode::Day, DayNightMode::Night];

    pub fn label(self) -> &'static str {
        match self {
            DayNightMode::Auto => "Auto",
            DayNightMode::Day => "Day",
            DayNightMode::Night => "Night",
        }
    }

    /// The hour used for rendering; overrides never affect time progression.
    pub fn visual_hour(self, hour: i32) -> f32 {
        match self {
            DayNightMode::Auto => hour as f32,
            DayNightMode::Day => 12.0,
            DayNightMode::Night => 22.0,
        }
    }
}

/// 0 in full daylight, 1 at night.
pub fn darkness(hour: f32) -> f32 {
    if (7.0..18.0).contains(&hour) {
        0.0
    } else if (5.0..7.0).contains(&hour) {
        1.0 - (hour - 5.0) / 2.0
    } else if (18.0..20.0).contains(&hour) {
        (hour - 18.0) / 2.0
    } else {
        1.0
    }
}

pub fn ambient_color(hour: f32) -> (f32, f32, f32) {
    if (7.0..18.0).contains(&hour) {
        (255.0, 255.0, 255.0)
    } else if (5.0..7.0).contains(&hour) {
        let t = (hour - 5.0) / 2.0;
        ((60.0 + 40.0 * t).round(), (40.0 + 30.0 * t).round(), (70.0 + 20.0 * t).round())
    } else if (18.0..20.0).contains(&hour) {
        let t = (hour - 18.0) / 2.0;
        ((100.0 - 40.0 * t).round(), (70.0 - 30.0 * t).round(), (90.0 - 20.0 * t).round())
    } else {
        (20.0, 30.0, 60.0)
    }
}

/// Deterministic window-lighting noise (same formula as the web version).
fn pseudo_random(seed: f64, n: f64) -> f64 {
    let s = (seed + n * 12.9898).sin() * 43758.5453;
    s - s.floor()
}

fn is_lit(kind: B) -> bool {
    !matches!(
        kind,
        B::Grass | B::Empty | B::Water | B::Road | B::Tree | B::Park | B::ParkLarge | B::Tennis
    )
}

fn window_count(kind: B) -> usize {
    match kind {
        B::Mansion | B::OfficeHigh => 4,
        B::ApartmentHigh | B::Mall => 5,
        B::HouseMedium | B::ShopMedium | B::FactoryMedium | B::ApartmentLow | B::OfficeLow => 3,
        _ => 2,
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Key {
    hour: f32,
    offset: (f32, f32),
    zoom: f32,
    size: (u32, u32),
    version: u64,
}

#[derive(Default)]
pub struct Lighting {
    image: Option<Arc<RenderImage>>,
    key: Option<Key>,
    garbage: Vec<Arc<RenderImage>>,
}

struct Layer {
    pixmap: Pixmap,
    transform: Transform,
}

impl Layer {
    fn circle(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, stops: Vec<GradientStop>, blend: BlendMode) {
        let Some(shader) = RadialGradient::new(
            Point::from_xy(cx, cy),
            Point::from_xy(cx, cy),
            rx,
            stops,
            SpreadMode::Pad,
            Transform::from_translate(cx, cy)
                .pre_scale(1.0, ry / rx)
                .pre_translate(-cx, -cy),
        ) else {
            return;
        };
        let mut pb = PathBuilder::new();
        pb.push_oval(tiny_skia::Rect::from_xywh(cx - rx, cy - ry, rx * 2.0, ry * 2.0).unwrap_or_else(|| {
            tiny_skia::Rect::from_xywh(cx, cy, 1.0, 1.0).expect("rect")
        }));
        let Some(path) = pb.finish() else {
            return;
        };
        let paint = Paint {
            shader,
            blend_mode: blend,
            anti_alias: true,
            ..Paint::default()
        };
        self.pixmap.fill_path(&path, &paint, FillRule::Winding, self.transform, None);
    }
}

fn white(a: f32) -> Color {
    Color::from_rgba(1.0, 1.0, 1.0, a.clamp(0.0, 1.0)).unwrap_or(Color::TRANSPARENT)
}

fn rgb(c: (f32, f32, f32), a: f32) -> Color {
    Color::from_rgba(c.0 / 255.0, c.1 / 255.0, c.2 / 255.0, a.clamp(0.0, 1.0)).unwrap_or(Color::TRANSPARENT)
}

fn stop(pos: f32, c: Color) -> GradientStop {
    GradientStop::new(pos, c)
}

impl Lighting {
    pub fn release_garbage(&mut self, window: &mut Window) {
        for img in self.garbage.drain(..) {
            let _ = window.drop_image(img);
        }
    }

    pub fn paint(&mut self, window: &mut Window, v: &View, state: &GameState, hour: f32) {
        let dark = darkness(hour);
        if dark <= 0.01 {
            if let Some(img) = self.image.take() {
                self.garbage.push(img);
            }
            self.key = None;
            return;
        }
        let size = (
            ((v.width * LAYER_SCALE).ceil() as u32).max(1),
            ((v.height * LAYER_SCALE).ceil() as u32).max(1),
        );
        let key = Key {
            hour,
            offset: (v.cam.offset_x, v.cam.offset_y),
            zoom: v.cam.zoom,
            size,
            version: state.game_version,
        };
        if self.key != Some(key) || self.image.is_none() {
            if let Some(img) = render_layer(v, state, hour, dark, size) {
                if let Some(old) = self.image.replace(img) {
                    self.garbage.push(old);
                }
            }
            self.key = Some(key);
        }
        if let Some(img) = &self.image {
            let b = v.bounds(v.origin_x, v.origin_y, size.0 as f32 / LAYER_SCALE, size.1 as f32 / LAYER_SCALE);
            let _ = window.paint_image(b, Corners::default(), img.clone(), 0, false);
        }
    }
}

fn render_layer(v: &View, state: &GameState, hour: f32, dark: f32, size: (u32, u32)) -> Option<Arc<RenderImage>> {
    let mut pixmap = Pixmap::new(size.0, size.1)?;
    let ambient = ambient_color(hour);
    pixmap.fill(rgb(ambient, dark * 0.6));
    let z = v.cam.zoom * LAYER_SCALE;
    let transform = Transform::from_row(z, 0.0, 0.0, z, v.cam.offset_x * LAYER_SCALE, v.cam.offset_y * LAYER_SCALE);
    let mut layer = Layer { pixmap, transform };
    let intensity = (dark * 1.3).min(1.0);

    let (vx, vy, vw, vh) = v.world_rect();
    let (left, right) = (vx - TILE_W * 2.0, vx + vw + TILE_W * 2.0);
    let (top, bottom) = (vy - TILE_H * 4.0, vy + vh + TILE_H * 4.0);

    let mut glows: Vec<(f32, f32, B)> = Vec::new();
    for t in state.grid.iter() {
        let (sx, sy) = grid_to_world(t.x as f32, t.y as f32);
        if sx + TILE_W < left || sx > right || sy + TILE_H * 3.0 < top || sy > bottom {
            continue;
        }
        let kind = t.building.kind;
        let (cx, cy) = (sx + TILE_W / 2.0, sy + TILE_H / 2.0);
        if kind == B::Road || kind == B::Bridge {
            layer.circle(
                cx,
                cy,
                28.0,
                28.0,
                vec![stop(0.0, white(0.75 * intensity)), stop(0.4, white(0.4 * intensity)), stop(1.0, white(0.0))],
                BlendMode::DestinationOut,
            );
            glows.push((cx, cy, kind));
        } else if is_lit(kind) && t.building.powered {
            let glow = if kind.is_commercial() {
                0.9
            } else if kind.is_residential() {
                0.65
            } else {
                0.75
            };
            let seed = (t.x * 1000 + t.y) as f64;
            let lit_chance = if kind.is_residential() { 0.55 } else { 0.75 };
            for i in 0..window_count(kind) {
                let i = i as f64;
                if pseudo_random(seed, i) >= lit_chance {
                    continue;
                }
                let wx = cx + (pseudo_random(seed, i + 10.0) as f32 - 0.5) * 22.0;
                let wy = cy - 18.0 + (pseudo_random(seed, i + 20.0) as f32 - 0.5) * 16.0;
                layer.circle(
                    wx,
                    wy,
                    12.5,
                    12.5,
                    vec![
                        stop(0.0, white(glow * intensity)),
                        stop(0.5, white(glow * 0.4 * intensity)),
                        stop(1.0, white(0.0)),
                    ],
                    BlendMode::DestinationOut,
                );
            }
            layer.circle(
                cx,
                cy + TILE_H / 4.0,
                TILE_W * 0.6,
                TILE_H / 2.5,
                vec![stop(0.0, white(0.28 * intensity)), stop(1.0, white(0.0))],
                BlendMode::DestinationOut,
            );
            if matches!(kind, B::Hospital | B::FireStation | B::PoliceStation | B::PowerPlant) {
                glows.push((cx, cy, kind));
            }
        }
    }

    for (cx, cy, kind) in glows {
        let (color, radius, y) = match kind {
            B::Road | B::Bridge => {
                layer.circle(
                    cx,
                    cy,
                    20.0,
                    20.0,
                    vec![
                        stop(0.0, rgb((255.0, 210.0, 130.0), 0.3 * intensity)),
                        stop(0.5, rgb((255.0, 190.0, 100.0), 0.15 * intensity)),
                        stop(1.0, Color::TRANSPARENT),
                    ],
                    BlendMode::SourceOver,
                );
                continue;
            }
            B::Hospital => ((255.0, 80.0, 80.0), 25.0, cy - 15.0),
            B::FireStation => ((255.0, 100.0, 50.0), 22.0, cy - 15.0),
            B::PoliceStation => ((60.0, 140.0, 255.0), 22.0, cy - 15.0),
            _ => ((255.0, 200.0, 50.0), 30.0, cy - 15.0),
        };
        layer.circle(
            cx,
            y,
            radius,
            radius,
            vec![
                stop(0.0, rgb(color, 0.55 * intensity)),
                stop(0.5, rgb(color, 0.25 * intensity)),
                stop(1.0, Color::TRANSPARENT),
            ],
            BlendMode::SourceOver,
        );
    }

    let (w, h) = size;
    let mut bgra = Vec::with_capacity((w * h * 4) as usize);
    for p in layer.pixmap.pixels() {
        let c = p.demultiply();
        let [r, g, b, a] = multiply_to_over(c.red(), c.green(), c.blue(), c.alpha());
        bgra.extend_from_slice(&[b, g, r, a]);
    }
    let buf = ImageBuffer::from_raw(w, h, bgra)?;
    Some(Arc::new(RenderImage::new(SmallVec::from_elem(Frame::new(buf), 1))))
}

/// Converts a pixel meant to be composited with `multiply` into one that
/// gives a similar result with ordinary alpha blending, assuming a mid-tone
/// scene behind it. Multiply darkens each channel to `S * (1 - a + a*C)`;
/// an "over" blend of colour `K` with alpha `a'` gives `S*(1-a') + K*a'`.
/// Returns straight-alpha `[r, g, b, a']`.
fn multiply_to_over(r: u8, g: u8, b: u8, a: u8) -> [u8; 4] {
    const SCENE: f32 = 0.55;
    let a = a as f32 / 255.0;
    let c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
    let min = c[0].min(c[1]).min(c[2]);
    let a2 = a * (1.0 - min);
    if a2 <= 0.002 {
        return [0, 0, 0, 0];
    }
    let k = |cc: f32| ((SCENE * (a2 - a + a * cc) / a2).clamp(0.0, 1.0) * 255.0) as u8;
    [k(c[0]), k(c[1]), k(c[2]), (a2 * 255.0) as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darkness_follows_the_web_schedule() {
        assert_eq!(darkness(12.0), 0.0);
        assert_eq!(darkness(22.0), 1.0);
        assert_eq!(darkness(6.0), 0.5);
        assert_eq!(darkness(19.0), 0.5);
        assert_eq!(DayNightMode::Night.visual_hour(12), 22.0);
        assert_eq!(DayNightMode::Day.visual_hour(23), 12.0);
    }

    #[test]
    fn multiply_conversion_is_transparent_for_white_and_dark_for_night() {
        assert_eq!(multiply_to_over(255, 255, 255, 200)[3], 0);
        let [r, g, b, a] = multiply_to_over(20, 30, 60, 153);
        assert!(a > 120 && b > r && g >= r);
    }
}
