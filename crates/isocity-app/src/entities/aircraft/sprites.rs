//! Plane types and their rotated sprites from the airplane sheet.

use crate::render::world::View;
use crate::sprites::config::Sheet;
use crate::sprites::SheetStore;
use gpui::{Corners, RenderImage, Window};
use image::{Frame, ImageBuffer};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};
use std::sync::Arc;
use tiny_skia::{Pixmap, PixmapPaint, Transform};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlaneType {
    B737,
    B777,
    B747,
    /// On the sprite sheet, but (as in the web version) never spawned.
    #[allow(dead_code)]
    A380,
    Seaplane,
    G650,
}

impl PlaneType {
    fn row(self) -> u32 {
        match self {
            PlaneType::B737 => 0,
            PlaneType::B777 => 1,
            PlaneType::B747 => 2,
            PlaneType::A380 => 3,
            PlaneType::Seaplane => 4,
            PlaneType::G650 => 5,
        }
    }

    pub(super) fn scale(self) -> f32 {
        match self {
            PlaneType::B737 => 0.152,
            PlaneType::B777 => 0.184,
            PlaneType::B747 => 0.196,
            PlaneType::A380 => 0.224,
            PlaneType::G650 => 0.112,
            PlaneType::Seaplane => 0.09,
        }
    }
}

pub(super) const PLANE_TYPES: [PlaneType; 7] = [
    PlaneType::B737,
    PlaneType::B737,
    PlaneType::B737,
    PlaneType::B777,
    PlaneType::B777,
    PlaneType::B747,
    PlaneType::G650,
];

/// (col, mirror_x, mirror_y, base_angle) for each of 8 headings.
fn direction_cell(t: PlaneType, angle: f32) -> (u32, bool, bool, f32) {
    let deg = angle.rem_euclid(TAU).to_degrees();
    let dir = ((deg + 22.5) / 45.0).floor() as i32 % 8; // 0=e,1=se,2=s,3=sw,4=w,5=nw,6=n,7=ne
    let override_col1 = matches!(t, PlaneType::Seaplane | PlaneType::G650);
    match dir {
        0 => (2, true, false, 0.0),
        1 if override_col1 => (3, true, true, 3.0 * PI / 4.0 - 0.78),
        1 => (0, true, false, PI / 4.0 - 0.26),
        2 => (3, false, true, PI / 2.0),
        3 => (0, false, false, 3.0 * PI / 4.0 + 0.26),
        4 => (2, false, false, PI),
        5 if override_col1 => (3, false, false, 3.0 * PI / 2.0),
        5 => (1, false, true, PI / 4.0 - 0.26),
        6 => (3, false, false, 3.0 * PI / 2.0),
        _ if override_col1 => (3, true, false, -PI / 4.0 - 0.69),
        _ => (1, false, false, -PI / 4.0 + 0.17),
    }
}

fn crop_adjust(t: PlaneType, col: u32, angle: f32) -> (f32, f32) {
    let deg = angle.rem_euclid(TAU).to_degrees();
    let sw_dir = (112.5..157.5).contains(&deg);
    match t {
        PlaneType::B737 => (0.0, if col == 3 { -70.0 } else { -8.0 }),
        PlaneType::B777 | PlaneType::B747 => {
            if col == 3 {
                (70.0, -70.0)
            } else {
                (8.0, 0.0)
            }
        }
        PlaneType::G650 => (12.0, 0.0),
        PlaneType::Seaplane => {
            if sw_dir {
                (40.0, -40.0)
            } else {
                (120.0, 0.0)
            }
        }
        PlaneType::A380 => (0.0, 0.0),
    }
}

struct PlaneSprite {
    image: Arc<RenderImage>,
    /// Half size in world units at scale 1 (before altitude scaling).
    half: f32,
}

#[derive(Default)]
pub struct PlaneSprites {
    cache: HashMap<(PlaneType, u32), Option<Arc<PlaneSprite>>>,
}

const HEADINGS: u32 = 48;

impl PlaneSprites {
    fn get(&mut self, store: &SheetStore, t: PlaneType, angle: f32) -> Option<Arc<PlaneSprite>> {
        let q = ((angle.rem_euclid(TAU) / TAU * HEADINGS as f32).round() as u32) % HEADINGS;
        if let Some(s) = self.cache.get(&(t, q)) {
            return s.clone();
        }
        let sheet = store.get(Sheet::Planes)?;
        let a = q as f32 / HEADINGS as f32 * TAU;
        let (col, mx, my, base) = direction_cell(t, a);
        let (top, bottom) = crop_adjust(t, col, a);
        let (iw, ih) = sheet.dimensions();
        let (cw, ch) = (iw as f32 / 5.0, ih as f32 / 6.0);
        let sx = col as f32 * cw;
        let sy = t.row() as f32 * ch + top;
        let sh = (ch - top - bottom).min(ih as f32 - sy);
        let (cx0, cy0, cw_i, ch_i) = (sx as u32, sy.max(0.0) as u32, cw as u32, sh.max(1.0) as u32);
        let mut cell = Pixmap::new(cw_i, ch_i)?;
        for yy in 0..ch_i {
            for xx in 0..cw_i {
                let (px, py) = (cx0 + xx, cy0 + yy);
                if px >= iw || py >= ih {
                    continue;
                }
                let [r, g, b, al] = sheet.get_pixel(px, py).0;
                cell.pixels_mut()[(yy * cw_i + xx) as usize] = tiny_skia::ColorU8::from_rgba(r, g, b, al).premultiply();
            }
        }
        let rot = {
            let mut d = a - base;
            while d > PI {
                d -= TAU;
            }
            while d < -PI {
                d += TAU;
            }
            d
        };
        let bake_scale = t.scale() * 4.0;
        let diag = ((cw_i * cw_i + ch_i * ch_i) as f32).sqrt();
        let side = (diag * bake_scale).ceil() as u32 + 2;
        let mut out = Pixmap::new(side, side)?;
        let tr = Transform::from_translate(side as f32 / 2.0, side as f32 / 2.0)
            .pre_rotate(rot.to_degrees())
            .pre_scale(if mx { -bake_scale } else { bake_scale }, if my { -bake_scale } else { bake_scale })
            .pre_translate(-(cw_i as f32) / 2.0, -(ch_i as f32) / 2.0);
        let paint = PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };
        out.draw_pixmap(0, 0, cell.as_ref(), &paint, tr, None);
        let mut bgra = Vec::with_capacity((side * side * 4) as usize);
        for p in out.pixels() {
            let c = p.demultiply();
            bgra.extend_from_slice(&[c.blue(), c.green(), c.red(), c.alpha()]);
        }
        let buf = ImageBuffer::from_raw(side, side, bgra)?;
        let sprite = Arc::new(PlaneSprite {
            image: Arc::new(RenderImage::new(SmallVec::from_elem(Frame::new(buf), 1))),
            half: side as f32 / 4.0 / 2.0,
        });
        self.cache.insert((t, q), Some(sprite.clone()));
        Some(sprite)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint(&mut self, window: &mut Window, v: &View, store: &SheetStore, t: PlaneType, x: f32, y: f32, angle: f32, scale: f32) {
        if let Some(s) = self.get(store, t, angle) {
            let half = s.half * scale;
            let (sx, sy) = v.to_screen(x - half, y - half);
            let size = half * 2.0 * v.cam.zoom;
            let _ = window.paint_image(v.bounds(sx, sy, size, size), Corners::default(), s.image.clone(), 0, false);
        }
    }
}
