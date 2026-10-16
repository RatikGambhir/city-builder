//! Small vector sprites (vehicles, boats, aircraft) baked with tiny-skia at a
//! handful of quantized rotations, then drawn as GPU images.

use crate::render::canvas2d::{Canvas, Rgba};
use crate::render::world::View;
use gpui::{Corners, RenderImage, Window};
use image::{Frame, ImageBuffer};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::sync::Arc;
use tiny_skia::Transform;

/// Supersampling factor for baked sprites (crisp up to ~4x zoom).
const SCALE: f32 = 4.0;
pub const ANGLE_STEPS: u32 = 48;

pub struct Baked {
    pub image: Arc<RenderImage>,
    /// Half extent in world units (sprite is a square centered on the anchor).
    pub radius: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shape {
    Car(u32),
    Bus(u32),
    FireTruck,
    PoliceCar,
    Locomotive(u32),
    PassengerCar(u32),
    BoxCar(u32),
    TankCar(u32),
    FlatCar(u32),
    Caboose(u32),
    Boat(u32),
    Barge(u32),
    Helicopter(u32),
}

pub fn quantize(angle: f32) -> u32 {
    let a = angle.rem_euclid(std::f32::consts::TAU);
    ((a / std::f32::consts::TAU * ANGLE_STEPS as f32).round() as u32) % ANGLE_STEPS
}

pub fn angle_of(q: u32) -> f32 {
    q as f32 / ANGLE_STEPS as f32 * std::f32::consts::TAU
}

#[derive(Default)]
pub struct BakeCache {
    map: HashMap<(Shape, u32), Arc<Baked>>,
}

impl BakeCache {
    pub fn get(&mut self, shape: Shape, angle: f32) -> Arc<Baked> {
        let q = quantize(angle);
        self.map
            .entry((shape, q))
            .or_insert_with(|| Arc::new(bake(shape, angle_of(q))))
            .clone()
    }

    /// Paints `shape` centered at world `(x, y)` rotated by `angle`.
    pub fn paint(&mut self, window: &mut Window, v: &View, shape: Shape, x: f32, y: f32, angle: f32) {
        let b = self.get(shape, angle);
        let (sx, sy) = v.to_screen(x - b.radius, y - b.radius);
        let s = b.radius * 2.0 * v.cam.zoom;
        let _ = window.paint_image(v.bounds(sx, sy, s, s), Corners::default(), b.image.clone(), 0, false);
    }
}

fn shade(c: u32, f: f32) -> Rgba {
    let r = (((c >> 16) & 0xff) as f32 * f).clamp(0.0, 255.0) as u8;
    let g = (((c >> 8) & 0xff) as f32 * f).clamp(0.0, 255.0) as u8;
    let b = ((c & 0xff) as f32 * f).clamp(0.0, 255.0) as u8;
    Rgba(r, g, b, 255)
}

fn rect(c: &mut Canvas, x: f32, y: f32, w: f32, h: f32, color: Rgba) {
    c.fill_poly(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], color);
}

fn bake(shape: Shape, angle: f32) -> Baked {
    let radius: f32 = match shape {
        Shape::Car(_) | Shape::PoliceCar => 8.0,
        Shape::Bus(_) | Shape::FireTruck => 14.0,
        Shape::Locomotive(_) => 16.0,
        Shape::PassengerCar(_) | Shape::Caboose(_) => 11.0,
        Shape::BoxCar(_) | Shape::TankCar(_) | Shape::FlatCar(_) => 16.0,
        Shape::Boat(_) => 10.0,
        Shape::Barge(_) => 30.0,
        Shape::Helicopter(_) => 14.0,
    };
    let px = (radius * 2.0 * SCALE).ceil() as u32;
    let t = Transform::from_scale(SCALE, SCALE)
        .pre_translate(radius, radius)
        .pre_rotate(angle.to_degrees());
    let mut c = Canvas::new(px, px, t).expect("bake canvas");
    match shape {
        Shape::Car(col) => {
            let s = 0.5;
            c.fill_poly(&[(-10.0 * s, -5.0 * s), (10.0 * s, -5.0 * s), (12.0 * s, 0.0), (10.0 * s, 5.0 * s), (-10.0 * s, 5.0 * s)], shade(col, 1.0));
            rect(&mut c, -4.0 * s, -2.8 * s, 7.0 * s, 5.6 * s, Rgba(255, 255, 255, 153));
            rect(&mut c, -10.0 * s, -4.0 * s, 2.4 * s, 8.0 * s, Rgba::hex(0x111827));
        }
        Shape::PoliceCar => {
            let s = 0.5;
            c.fill_poly(&[(-10.0 * s, -5.0 * s), (10.0 * s, -5.0 * s), (12.0 * s, 0.0), (10.0 * s, 5.0 * s), (-10.0 * s, 5.0 * s)], Rgba::hex(0x1e3a8a));
            rect(&mut c, -4.0 * s, -5.0 * s, 7.0 * s, 10.0 * s, Rgba::hex(0xffffff));
            rect(&mut c, -1.0 * s, -2.0 * s, 2.0 * s, 4.0 * s, Rgba::hex(0xef4444));
        }
        Shape::Bus(col) => {
            let s = 0.6;
            let (l, w) = (20.0 * s, 7.0 * s);
            rect(&mut c, -l, -w, l * 2.0, w * 2.0, shade(col, 1.0));
            rect(&mut c, -l * 0.8, -w * 0.7, l * 1.4, w * 0.9, Rgba(255, 255, 255, 64));
            for i in 0..4 {
                let wx = -l * 0.7 + i as f32 * l * 0.45;
                rect(&mut c, wx, -w * 0.55, l * 0.25, w * 1.1, Rgba(191, 219, 254, 204));
            }
            rect(&mut c, -l * 0.95, -w * 0.9, l * 0.1, w * 1.8, Rgba::hex(0x111827));
            rect(&mut c, l * 0.85, -w * 0.35, l * 0.1, w * 0.7, Rgba::hex(0xfbbf24));
        }
        Shape::FireTruck => {
            let s = 0.6;
            let (l, w) = (16.0 * s, 6.0 * s);
            rect(&mut c, -l, -w, l * 2.0, w * 2.0, Rgba::hex(0xdc2626));
            rect(&mut c, -l * 0.8, -w * 0.4, l * 1.3, w * 0.8, Rgba::hex(0xd1d5db));
            rect(&mut c, l * 0.55, -w * 0.8, l * 0.35, w * 1.6, Rgba(191, 219, 254, 220));
        }
        Shape::Locomotive(col) | Shape::PassengerCar(col) | Shape::BoxCar(col) | Shape::TankCar(col) | Shape::FlatCar(col) | Shape::Caboose(col) => {
            let (len, w) = match shape {
                Shape::Locomotive(_) => (25.0, 4.3),
                Shape::PassengerCar(_) | Shape::Caboose(_) => (16.0, 4.3),
                _ => (28.0, 4.3),
            };
            let (hl, hw) = (len / 2.0 * 0.9, w * 0.9);
            // Undercarriage shadow
            rect(&mut c, -hl, -hw + 0.6, hl * 2.0, hw * 2.0, Rgba(0, 0, 0, 70));
            match shape {
                Shape::Locomotive(_) => {
                    rect(&mut c, -hl, -hw, hl * 2.0, hw * 2.0, shade(col, 1.0));
                    rect(&mut c, hl * 0.35, -hw * 0.85, hl * 0.55, hw * 1.7, shade(col, 0.7));
                    rect(&mut c, hl * 0.5, -hw * 0.6, hl * 0.3, hw * 1.2, Rgba(191, 219, 254, 230));
                    rect(&mut c, -hl * 0.9, -hw * 0.3, hl * 1.1, hw * 0.6, shade(col, 1.25));
                    rect(&mut c, hl * 0.9, -hw * 0.5, hl * 0.1, hw, Rgba::hex(0xfbbf24));
                }
                Shape::PassengerCar(_) => {
                    rect(&mut c, -hl, -hw, hl * 2.0, hw * 2.0, shade(col, 1.0));
                    for i in 0..4 {
                        let wx = -hl * 0.8 + i as f32 * hl * 0.45;
                        rect(&mut c, wx, -hw * 0.6, hl * 0.25, hw * 1.2, Rgba(191, 219, 254, 230));
                    }
                }
                Shape::BoxCar(_) => {
                    rect(&mut c, -hl, -hw, hl * 2.0, hw * 2.0, shade(col, 1.0));
                    for i in 0..5 {
                        let x = -hl + (i as f32 + 0.5) * hl * 0.4;
                        rect(&mut c, x, -hw, 0.4, hw * 2.0, shade(col, 0.7));
                    }
                }
                Shape::TankCar(_) => {
                    rect(&mut c, -hl, -hw * 0.6, hl * 2.0, hw * 1.2, shade(col, 0.6));
                    c.fill_ellipse(0.0, 0.0, hl * 0.95, hw * 0.95, shade(col, 1.0));
                    c.fill_ellipse(0.0, -hw * 0.3, hl * 0.85, hw * 0.3, shade(col, 1.3));
                }
                Shape::FlatCar(_) => {
                    rect(&mut c, -hl, -hw * 0.8, hl * 2.0, hw * 1.6, Rgba::hex(0x4b5563));
                    rect(&mut c, -hl * 0.8, -hw * 0.7, hl * 0.7, hw * 1.4, shade(col, 1.0));
                    rect(&mut c, hl * 0.05, -hw * 0.7, hl * 0.7, hw * 1.4, shade(col, 0.8));
                }
                _ => {
                    rect(&mut c, -hl, -hw, hl * 2.0, hw * 2.0, Rgba::hex(0xb91c1c));
                    rect(&mut c, -hl * 0.3, -hw * 0.7, hl * 0.6, hw * 1.4, Rgba::hex(0x7f1d1d));
                }
            }
        }
        Shape::Boat(col) => {
            c.fill_poly(&[(-7.0, -2.6), (4.0, -2.6), (8.0, 0.0), (4.0, 2.6), (-7.0, 2.6)], shade(col, 1.0));
            c.fill_poly(&[(-6.0, -1.6), (3.0, -1.6), (6.0, 0.0), (3.0, 1.6), (-6.0, 1.6)], Rgba::hex(0xf3f4f6));
            rect(&mut c, -3.5, -1.3, 4.0, 2.6, shade(col, 0.8));
            rect(&mut c, -2.5, -0.9, 2.0, 1.8, Rgba(147, 197, 253, 220));
        }
        Shape::Barge(col) => {
            c.fill_poly(&[(-24.0, -6.0), (18.0, -6.0), (25.0, 0.0), (18.0, 6.0), (-24.0, 6.0)], shade(col, 1.0));
            c.fill_poly(&[(-23.0, -5.0), (17.0, -5.0), (22.0, 0.0), (17.0, 5.0), (-23.0, 5.0)], shade(col, 0.75));
            let colors = [0xc0392b, 0x2980b9, 0xf39c12, 0x27ae60, 0x8e44ad, 0xd35400];
            for i in 0..5 {
                for j in 0..2 {
                    let cc = colors[((i * 2 + j) as usize + col as usize) % colors.len()];
                    rect(&mut c, -14.0 + i as f32 * 6.0, -4.2 + j as f32 * 4.3, 5.4, 3.9, Rgba::hex(cc));
                }
            }
            rect(&mut c, -23.0, -4.5, 6.0, 9.0, Rgba::hex(0xecf0f1));
            rect(&mut c, -21.5, -3.0, 3.0, 6.0, Rgba::hex(0x34495e));
        }
        Shape::Helicopter(col) => {
            c.fill_ellipse(0.0, 0.0, 6.5, 3.4, shade(col, 1.0));
            c.fill_ellipse(2.5, 0.0, 2.5, 2.2, Rgba(147, 197, 253, 230));
            rect(&mut c, -13.0, -0.7, 8.0, 1.4, shade(col, 0.85));
            rect(&mut c, -13.5, -2.5, 1.5, 5.0, shade(col, 0.7));
            rect(&mut c, -4.0, 3.6, 9.0, 0.7, Rgba::hex(0x374151));
            rect(&mut c, -4.0, -4.3, 9.0, 0.7, Rgba::hex(0x374151));
            if col == 0xffffff {
                rect(&mut c, -2.4, -0.5, 2.2, 1.0, Rgba::hex(0xdc2626));
                rect(&mut c, -1.8, -1.1, 1.0, 2.2, Rgba::hex(0xdc2626));
            }
        }
    }
    let (w, h, bgra) = c.into_bgra();
    let buf = ImageBuffer::from_raw(w, h, bgra).expect("buffer");
    Baked {
        image: Arc::new(RenderImage::new(SmallVec::from_elem(Frame::new(buf), 1))),
        radius,
    }
}
