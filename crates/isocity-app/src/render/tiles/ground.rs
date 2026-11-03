//! Base tiles: grass, pavement, foundations, water and beaches.

use crate::render::canvas2d::{Canvas, Rgba, P};
use crate::iso::{grid_to_world, TILE_H, TILE_W};
use isocity_core::{BuildingType as B, ZoneType};
use super::{PaintCtx, diamond, norm};
use super::palette::{BEACH_CURB, BEACH_FILL};

// ---------------------------------------------------------------------------
// Base tiles
// ---------------------------------------------------------------------------

pub fn draw_base_tile(c: &mut Canvas, pc: &PaintCtx, x: i32, y: i32, skip_green: bool) {
    let t = pc.map.get(x, y);
    let (sx, sy) = grid_to_world(x as f32, y as f32);
    let built = !matches!(t.kind, B::Grass | B::Empty);
    let (top, stroke) = if t.kind == B::Water {
        (0x2563eb, 0x1e3a8a)
    } else if matches!(t.kind, B::Road | B::Bridge) {
        (0x4a4a4a, 0x333333)
    } else if t.park {
        (0x4a7c3f, 0x2d4a26)
    } else {
        match t.zone {
            ZoneType::Residential => (if built { 0x3d7c3f } else { 0x2d5a2d }, 0x22c55e),
            ZoneType::Commercial => (if built { 0x3a5c7c } else { 0x2a4a6a }, 0x3b82f6),
            ZoneType::Industrial => (if built { 0x7c5c3a } else { 0x6a4a2a }, 0xf59e0b),
            ZoneType::None => (0x4a7c3f, 0x2d4a26),
        }
    };
    let skip = (skip_green && matches!(t.kind, B::Grass | B::Empty | B::Tree)) || t.kind == B::Bridge;
    if skip {
        return;
    }
    let d = diamond(sx, sy);
    c.fill_poly(&d, Rgba::hex(top));
    if pc.zoom >= 0.6 {
        c.stroke_poly(&d, true, 0.5, Rgba::hex(stroke), None);
    }
    if t.zone != ZoneType::None && pc.zoom >= 0.95 && matches!(t.kind, B::Grass | B::Empty) {
        c.stroke_poly(&d, true, 1.5, Rgba::hex(stroke), Some(&[4.0, 2.0]));
    }
}

pub fn draw_green_base(c: &mut Canvas, pc: &PaintCtx, x: i32, y: i32) {
    let t = pc.map.get(x, y);
    let (sx, sy) = grid_to_world(x as f32, y as f32);
    let (top, stroke) = match t.zone {
        ZoneType::None => (0x4a7c3f, 0x2d4a26),
        ZoneType::Residential => (0x2d5a2d, 0x22c55e),
        ZoneType::Commercial => (0x2a4a6a, 0x3b82f6),
        ZoneType::Industrial => (0x6a4a2a, 0xf59e0b),
    };
    let d = diamond(sx, sy);
    c.fill_poly(&d, Rgba::hex(top));
    if pc.zoom >= 0.6 {
        c.stroke_poly(&d, true, 0.5, Rgba::hex(stroke), None);
    }
    if t.zone != ZoneType::None && pc.zoom >= 0.95 {
        c.stroke_poly(&d, true, 1.5, Rgba::hex(stroke), Some(&[4.0, 2.0]));
    }
}

pub fn draw_grey_base(c: &mut Canvas, pc: &PaintCtx, x: i32, y: i32) {
    let (sx, sy) = grid_to_world(x as f32, y as f32);
    let d = diamond(sx, sy);
    c.fill_poly(&d, Rgba::hex(0x6b7280));
    if pc.zoom >= 0.6 {
        c.stroke_poly(&d, true, 0.5, Rgba::hex(0x374151), None);
    }
}

pub fn draw_foundation(c: &mut Canvas, pc: &PaintCtx, x: i32, y: i32) {
    let (sx, sy) = grid_to_world(x as f32, y as f32);
    let d = diamond(sx, sy);
    c.fill_poly(&d, Rgba::hex(0xa67c52));
    if pc.zoom >= 0.8 {
        for i in 0..10 {
            let seed = ((sx * 17.0 + sy * 31.0 + i as f32 * 7.0) as i64).rem_euclid(100) as f32;
            let ox = ((seed % 40.0) - 20.0) / 100.0 * TILE_W;
            let oy = (((seed * 3.0) % 40.0) - 20.0) / 100.0 * TILE_H;
            c.fill_circle(sx + TILE_W / 2.0 + ox, sy + TILE_H / 2.0 + oy, 0.67, Rgba::hex(0x8b6914));
        }
    }
    if pc.zoom >= 0.6 {
        c.stroke_poly(&d, true, 0.5, Rgba::hex(0x6b4423), None);
    }
}

// ---------------------------------------------------------------------------
// Water & beaches
// ---------------------------------------------------------------------------

fn water_seed(x: i32, y: i32) -> (f32, f32) {
    let sx = ((x as i64 * 7919 + y as i64 * 6271).rem_euclid(1000)) as f32 / 1000.0;
    let sy = ((x as i64 * 4177 + y as i64 * 9311).rem_euclid(1000)) as f32 / 1000.0;
    (sx, sy)
}

/// Water texture inside the (slightly expanded) diamond.
pub fn draw_water(c: &mut Canvas, pc: &PaintCtx, x: i32, y: i32) {
    let (sx, sy) = grid_to_world(x as f32, y as f32);
    let m = pc.map;
    let Some(img) = pc.water else {
        c.fill_poly(&diamond(sx, sy), Rgba::hex(0x0ea5e9));
        return;
    };
    let (n, e, s, w) = (m.is_water(x - 1, y), m.is_water(x, y - 1), m.is_water(x + 1, y), m.is_water(x, y + 1));
    let count = n as u8 + e as u8 + s as u8 + w as u8;
    let (imw, imh) = (img.width() as f32, img.height() as f32);
    let (seed_x, seed_y) = water_seed(x, y);
    let crop_w = imw * 0.35;
    let crop_h = imh * 0.35;
    let src = (seed_x * (imw - crop_w), seed_y * (imh - crop_h), crop_w, crop_h);
    let aspect = crop_h / crop_w;
    let expand = TILE_W * 0.4;
    let cx = sx + TILE_W / 2.0;
    let cy = sy + TILE_H / 2.0;
    let top_y = sy - if n && e { expand * 0.5 } else { 0.0 } - if n && e { expand * 0.3 } else { 0.0 };
    let right_x = sx + TILE_W + if e && s { expand * 0.8 } else { 0.0 };
    let bottom_y = sy + TILE_H + if s && w { expand * 0.8 } else { 0.0 };
    let left_x = sx - if w && n { expand * 0.8 } else { 0.0 };
    let clip = [(cx, top_y), (right_x, cy), (cx, bottom_y), (left_x, cy)];
    let jx = (seed_x - 0.5) * TILE_W * 0.3;
    let jy = (seed_y - 0.5) * TILE_H * 0.3;
    if pc.zoom < 0.5 {
        let dw = TILE_W * 1.15;
        let dh = dw * aspect;
        c.fill_poly_with_image(&clip, img.as_ref(), src, (cx - dw / 2.0, cy - dh / 2.0, dw, dh), 0.9);
    } else if count >= 2 {
        let ow = TILE_W * (2.0 + count as f32 * 0.3);
        let oh = ow * aspect;
        c.fill_poly_with_image(&clip, img.as_ref(), src, (cx - ow / 2.0 + jx, cy - oh / 2.0 + jy, ow, oh), 0.35);
        let cw = TILE_W * 1.1;
        let ch = cw * aspect;
        c.fill_poly_with_image(&clip, img.as_ref(), src, (cx - cw / 2.0 + jx * 0.5, cy - ch / 2.0 + jy * 0.5, cw, ch), 0.9);
    } else {
        let dw = TILE_W * 1.15;
        let dh = dw * aspect;
        c.fill_poly_with_image(&clip, img.as_ref(), src, (cx - dw / 2.0 + jx * 0.3, cy - dh / 2.0 + jy * 0.3, dw, dh), 0.95);
    }
}

/// Plain water under bridges and waterfront buildings.
pub fn draw_water_under(c: &mut Canvas, pc: &PaintCtx, x: i32, y: i32) {
    let (sx, sy) = grid_to_world(x as f32, y as f32);
    let d = diamond(sx, sy);
    c.fill_poly(&d, Rgba::hex(0x2563eb));
    if let Some(img) = pc.water {
        let (imw, imh) = (img.width() as f32, img.height() as f32);
        let (seed_x, seed_y) = water_seed(x, y);
        let cw = imw * 0.35;
        let ch = imh * 0.35;
        let src = (seed_x * (imw - cw), seed_y * (imh - ch), cw, ch);
        let dw = TILE_W * 1.15;
        let dh = dw * ch / cw;
        let jx = (seed_x - 0.5) * TILE_W * 0.3;
        let jy = (seed_y - 0.5) * TILE_H * 0.3;
        let (cx, cy) = (sx + TILE_W / 2.0, sy + TILE_H / 2.0);
        c.fill_poly_with_image(&d, img.as_ref(), src, (cx - dw / 2.0 + jx * 0.3, cy - dh / 2.0 + jy * 0.3, dw, dh), 0.95);
    }
}

pub(super) const INWARD_N: P = (0.707, 0.707);

pub(super) const INWARD_E: P = (-0.707, 0.707);

pub(super) const INWARD_S: P = (-0.707, -0.707);

pub(super) const INWARD_W: P = (0.707, -0.707);

pub(super) fn shortened_inner(corner: P, other: P, inward: P, width: f32) -> P {
    let shorten = width * 0.707;
    let (dx, dy) = norm(corner.0 - other.0, corner.1 - other.1);
    let ox = corner.0 - dx * shorten;
    let oy = corner.1 - dy * shorten;
    (ox + inward.0 * width, oy + inward.1 * width)
}

/// Sandy strip on a water tile along edges that face land.
pub fn draw_beach_on_water(c: &mut Canvas, pc: &PaintCtx, x: i32, y: i32) {
    let m = pc.map;
    let land = |xx: i32, yy: i32| {
        let t = m.get(xx, yy);
        t.in_bounds && t.kind != B::Water && !t.marina_pier && t.kind != B::Bridge
    };
    let (n, e, s, w) = (land(x - 1, y), land(x, y - 1), land(x + 1, y), land(x, y + 1));
    if !(n || e || s || w) {
        return;
    }
    let width = TILE_W * 0.04 * 2.5;
    let (sx, sy) = grid_to_world(x as f32, y as f32);
    let [top, right, bottom, left] = diamond(sx, sy);
    let edge = |c: &mut Canvas, a: P, b: P, inward: P, sh_a: bool, sh_b: bool| {
        let shorten = width * 0.707;
        let (dx, dy) = norm(b.0 - a.0, b.1 - a.1);
        let a2 = if sh_a { (a.0 + dx * shorten, a.1 + dy * shorten) } else { a };
        let b2 = if sh_b { (b.0 - dx * shorten, b.1 - dy * shorten) } else { b };
        let ai = (a2.0 + inward.0 * width, a2.1 + inward.1 * width);
        let bi = (b2.0 + inward.0 * width, b2.1 + inward.1 * width);
        c.fill_poly(&[a2, b2, bi, ai], BEACH_FILL);
        c.line(ai, bi, 1.5, BEACH_CURB);
    };
    if n {
        edge(c, left, top, INWARD_N, w, e);
    }
    if e {
        edge(c, top, right, INWARD_E, n, s);
    }
    if s {
        edge(c, right, bottom, INWARD_S, e, w);
    }
    if w {
        edge(c, bottom, left, INWARD_W, s, n);
    }
    let corner = |c: &mut Canvas, p: P, o1: P, i1: P, o2: P, i2: P| {
        let a = shortened_inner(p, o1, i1, width);
        let b = shortened_inner(p, o2, i2, width);
        c.fill_poly(&[p, a, b], BEACH_FILL);
    };
    if n && e {
        corner(c, top, left, INWARD_N, right, INWARD_E);
    }
    if e && s {
        corner(c, right, top, INWARD_E, bottom, INWARD_S);
    }
    if s && w {
        corner(c, bottom, right, INWARD_S, left, INWARD_W);
    }
    if w && n {
        corner(c, left, bottom, INWARD_W, top, INWARD_N);
    }
}
