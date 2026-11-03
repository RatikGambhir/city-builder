//! Rail track geometry and painting.

use crate::render::canvas2d::{Canvas, Rgba, P};
use crate::iso::{grid_to_world, TILE_H, TILE_W};
use tiny_skia::LineCap;
use super::{PaintCtx, diamond, norm};
use super::palette::{BALLAST, RAIL, RAIL_SHADOW, TIE};

// ---------------------------------------------------------------------------
// Rail
// ---------------------------------------------------------------------------

const ISO_NS: P = (0.894427, 0.447214);

const ISO_EW: P = (-0.894427, 0.447214);

const NEG_ISO_EW: P = (0.894427, -0.447214);

const NEG_ISO_NS: P = (-0.894427, -0.447214);

/// One track centerline (start, control, end) with perpendiculars.
struct TrackSeg {
    from: P,
    to: P,
    ctrl: Option<P>,
    from_perp: P,
    to_perp: P,
}

fn rail_segments(x: f32, y: f32, n: bool, e: bool, s: bool, w: bool) -> (Vec<TrackSeg>, bool) {
    let (tw, th) = (TILE_W, TILE_H);
    let c = (x + tw / 2.0, y + th / 2.0);
    let ne = (x + tw * 0.25, y + th * 0.25);
    let ee = (x + tw * 0.75, y + th * 0.25);
    let se = (x + tw * 0.75, y + th * 0.75);
    let we = (x + tw * 0.25, y + th * 0.75);
    let straight = |a: P, b: P, perp: P| TrackSeg { from: a, to: b, ctrl: None, from_perp: perp, to_perp: perp };
    let curve = |a: P, b: P, fp: P, tp: P, _cp: P| TrackSeg { from: a, to: b, ctrl: Some(c), from_perp: fp, to_perp: tp };
    let count = n as u8 + e as u8 + s as u8 + w as u8;
    let mut segs = Vec::new();
    let mut center = false;
    match (n, e, s, w) {
        _ if count == 4 => {
            segs.push(straight(ne, se, ISO_EW));
            segs.push(straight(ee, we, ISO_NS));
            center = true;
        }
        (false, true, true, true) => {
            segs.push(straight(ee, we, ISO_NS));
            segs.push(curve(se, ee, ISO_EW, NEG_ISO_NS, (-1.0, 0.0)));
            segs.push(curve(se, we, NEG_ISO_EW, NEG_ISO_NS, (0.0, -1.0)));
            center = true;
        }
        (true, false, true, true) => {
            segs.push(straight(ne, se, ISO_EW));
            segs.push(curve(we, ne, ISO_NS, NEG_ISO_EW, (1.0, 0.0)));
            segs.push(curve(we, se, NEG_ISO_NS, NEG_ISO_EW, (0.0, -1.0)));
            center = true;
        }
        (true, true, false, true) => {
            segs.push(straight(ee, we, ISO_NS));
            segs.push(curve(ne, ee, ISO_EW, ISO_NS, (0.0, 1.0)));
            segs.push(curve(ne, we, NEG_ISO_EW, ISO_NS, (1.0, 0.0)));
            center = true;
        }
        (true, true, true, false) => {
            segs.push(straight(ne, se, ISO_EW));
            segs.push(curve(ee, ne, ISO_NS, ISO_EW, (0.0, 1.0)));
            segs.push(curve(ee, se, ISO_NS, NEG_ISO_EW, (0.0, -1.0)));
            center = true;
        }
        (true, false, true, false) => segs.push(straight(ne, se, ISO_EW)),
        (false, true, false, true) => segs.push(straight(ee, we, ISO_NS)),
        (true, true, false, false) => segs.push(curve(ne, ee, ISO_EW, ISO_NS, (0.0, 1.0))),
        (true, false, false, true) => segs.push(curve(ne, we, NEG_ISO_EW, ISO_NS, (1.0, 0.0))),
        (false, true, true, false) => segs.push(curve(se, ee, ISO_EW, NEG_ISO_NS, (-1.0, 0.0))),
        (false, false, true, true) => segs.push(curve(se, we, NEG_ISO_EW, NEG_ISO_NS, (0.0, -1.0))),
        (true, false, false, false) => {
            segs.push(straight(c, ne, ISO_EW));
            center = true;
        }
        (false, true, false, false) => {
            segs.push(straight(c, ee, ISO_NS));
            center = true;
        }
        (false, false, true, false) => {
            segs.push(straight(c, se, ISO_EW));
            center = true;
        }
        (false, false, false, true) => {
            segs.push(straight(c, we, ISO_NS));
            center = true;
        }
        _ => {
            let nsd = norm(se.0 - ne.0, se.1 - ne.1);
            let ewd = norm(we.0 - ee.0, we.1 - ee.1);
            let len = ((se.0 - ne.0).powi(2) + (se.1 - ne.1).powi(2)).sqrt() * 0.35;
            segs.push(straight((c.0 - nsd.0 * len, c.1 - nsd.1 * len), (c.0 + nsd.0 * len, c.1 + nsd.1 * len), ewd));
        }
    }
    (segs, center)
}

pub(super) fn offset(p: P, d: P, a: f32) -> P {
    (p.0 + d.0 * a, p.1 + d.1 * a)
}

fn mid_perp(a: P, b: P) -> P {
    norm((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

/// Samples a (possibly curved) track offset sideways by `lateral`.
fn sample_track(seg: &TrackSeg, lateral: f32, t: f32) -> (P, P) {
    match seg.ctrl {
        None => {
            let a = offset(seg.from, seg.from_perp, lateral);
            let b = offset(seg.to, seg.to_perp, lateral);
            let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            (p, norm(b.0 - a.0, b.1 - a.1))
        }
        Some(ctrl) => {
            let mp = mid_perp(seg.from_perp, seg.to_perp);
            let a = offset(seg.from, seg.from_perp, lateral);
            let b = offset(seg.to, seg.to_perp, lateral);
            let cc = offset(ctrl, mp, lateral);
            let u = 1.0 - t;
            let p = (u * u * a.0 + 2.0 * u * t * cc.0 + t * t * b.0, u * u * a.1 + 2.0 * u * t * cc.1 + t * t * b.1);
            let d = (2.0 * u * (cc.0 - a.0) + 2.0 * t * (b.0 - cc.0), 2.0 * u * (cc.1 - a.1) + 2.0 * t * (b.1 - cc.1));
            (p, norm(d.0, d.1))
        }
    }
}

fn track_path(seg: &TrackSeg, lateral: f32, shift: P) -> Option<tiny_skia::Path> {
    let a = offset(seg.from, seg.from_perp, lateral);
    let b = offset(seg.to, seg.to_perp, lateral);
    let a = (a.0 + shift.0, a.1 + shift.1);
    let b = (b.0 + shift.0, b.1 + shift.1);
    match seg.ctrl {
        None => {
            let mut pb = tiny_skia::PathBuilder::new();
            pb.move_to(a.0, a.1);
            pb.line_to(b.0, b.1);
            pb.finish()
        }
        Some(ctrl) => {
            let cc = offset(ctrl, mid_perp(seg.from_perp, seg.to_perp), lateral);
            Canvas::quad_path(a, (cc.0 + shift.0, cc.1 + shift.1), b)
        }
    }
}

fn track_band(seg: &TrackSeg, lateral: f32, half_w: f32) -> Vec<P> {
    let steps = if seg.ctrl.is_some() { 10 } else { 1 };
    let mut left = Vec::new();
    let mut right = Vec::new();
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let (p, d) = sample_track(seg, lateral, t);
        let perp = (-d.1, d.0);
        left.push(offset(p, perp, half_w));
        right.push(offset(p, perp, -half_w));
    }
    right.reverse();
    left.extend(right);
    left
}

/// Ballast, ties and steel rails for a double-track tile.
pub fn draw_rail_track(c: &mut Canvas, pc: &PaintCtx, gx: i32, gy: i32, inset_only: bool) {
    let m = pc.map;
    let (x, y) = grid_to_world(gx as f32, gy as f32);
    let (n, e, s, w) = (m.has_rail(gx - 1, gy), m.has_rail(gx, gy - 1), m.has_rail(gx + 1, gy), m.has_rail(gx, gy + 1));
    let (segs, center) = rail_segments(x, y, n, e, s, w);
    let half_sep = TILE_W * 0.22 / 2.0;
    let half_ballast = TILE_W * 0.18 / 2.0;
    let half_gauge = TILE_W * 0.06 / 2.0;
    let (cx, cy) = (x + TILE_W / 2.0, y + TILE_H / 2.0);

    if !inset_only {
        for seg in &segs {
            for side in [half_sep, -half_sep] {
                c.fill_poly(&track_band(seg, side, half_ballast), BALLAST);
            }
        }
        if center {
            let size = (TILE_W * 0.18 + TILE_W * 0.22) * 0.8;
            let r = TILE_H / TILE_W;
            c.fill_poly(&[(cx, cy - size * r), (cx + size, cy), (cx, cy + size * r), (cx - size, cy)], BALLAST);
        }
        if pc.zoom >= 0.5 {
            let tie_len = TILE_W * 0.18 * 0.765 / 2.0;
            let tie_w = TILE_W * 0.018 / 2.0;
            for seg in &segs {
                for side in [half_sep, -half_sep] {
                    let n_ties = 7;
                    for i in 0..n_ties {
                        let t = (i as f32 + 0.5) / n_ties as f32;
                        let (p, d) = sample_track(seg, side, t);
                        let perp = (-d.1, d.0);
                        c.fill_poly(
                            &[
                                (p.0 + perp.0 * tie_len + d.0 * tie_w, p.1 + perp.1 * tie_len + d.1 * tie_w),
                                (p.0 + perp.0 * tie_len - d.0 * tie_w, p.1 + perp.1 * tie_len - d.1 * tie_w),
                                (p.0 - perp.0 * tie_len - d.0 * tie_w, p.1 - perp.1 * tie_len - d.1 * tie_w),
                                (p.0 - perp.0 * tie_len + d.0 * tie_w, p.1 - perp.1 * tie_len + d.1 * tie_w),
                            ],
                            TIE,
                        );
                    }
                }
            }
        }
    }
    let rail_w = if pc.zoom >= 0.7 { 0.85 } else { 0.7 };
    for seg in &segs {
        for side in [half_sep, -half_sep] {
            for g in [half_gauge, -half_gauge] {
                if let Some(p) = track_path(seg, side + g, (0.3, 0.3)) {
                    c.stroke_path(&p, rail_w + 0.3, if inset_only { Rgba::hex(0x2a2a2a) } else { RAIL_SHADOW }, None, LineCap::Round);
                }
                if let Some(p) = track_path(seg, side + g, (0.0, 0.0)) {
                    c.stroke_path(&p, rail_w, RAIL, None, LineCap::Round);
                }
            }
        }
    }
}

pub fn draw_rail_tile(c: &mut Canvas, pc: &PaintCtx, gx: i32, gy: i32) {
    let (x, y) = grid_to_world(gx as f32, gy as f32);
    let d = diamond(x, y);
    c.fill_poly(&d, Rgba::hex(0x5b6345));
    c.stroke_poly(&[d[2], d[3], d[0]], false, 1.0, Rgba::hex(0x4b5335), None);
    draw_rail_track(c, pc, gx, gy, false);
}
