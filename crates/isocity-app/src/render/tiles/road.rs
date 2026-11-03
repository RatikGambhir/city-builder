//! Road tiles with sidewalks, lane markings and merged avenues.

use crate::render::canvas2d::{Canvas, Rgba, P};
use crate::iso::{grid_to_world, TILE_H, TILE_W};
use tiny_skia::LineCap;
use super::{PaintCtx, diamond, norm};
use super::ground::{INWARD_E, INWARD_N, INWARD_S, INWARD_W, shortened_inner};
use super::palette::{ASPHALT, CENTER_LINE, CURB, LANE_MARKING, SIDEWALK};
use super::road_merge::{RoadKind, RoadOrient, Side, analyze_merged_road};

// ---------------------------------------------------------------------------
// Roads
// ---------------------------------------------------------------------------

pub fn draw_road(c: &mut Canvas, pc: &PaintCtx, gx: i32, gy: i32) {
    let m = pc.map;
    let (x, y) = grid_to_world(gx as f32, gy as f32);
    let (w, h) = (TILE_W, TILE_H);
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    c.fill_poly(&diamond(x, y), ASPHALT);

    let north = m.has_road(gx - 1, gy);
    let east = m.has_road(gx, gy - 1);
    let south = m.has_road(gx + 1, gy);
    let west = m.has_road(gx, gy + 1);
    let mi = analyze_merged_road(m, gx, gy);
    let road_w = w * match mi.kind {
        RoadKind::Highway => 0.16,
        RoadKind::Avenue => 0.15,
        RoadKind::Single => 0.14,
    };
    let sw = w * 0.08;
    let edge_stop = 0.98;
    let ne = (x + w * 0.25, y + h * 0.25);
    let ee = (x + w * 0.75, y + h * 0.25);
    let se = (x + w * 0.75, y + h * 0.75);
    let we = (x + w * 0.25, y + h * 0.75);
    let nd = norm(ne.0 - cx, ne.1 - cy);
    let ed = norm(ee.0 - cx, ee.1 - cy);
    let sd = norm(se.0 - cx, se.1 - cy);
    let wd = norm(we.0 - cx, we.1 - cy);
    let [top, right, bottom, left] = diamond(x, y);
    let show_sidewalks = pc.zoom >= 0.5;

    let outer = |dir: char| -> bool {
        if mi.kind == RoadKind::Single {
            return true;
        }
        match (mi.orient, dir) {
            (RoadOrient::Ns, 'e') => mi.side == Side::Right,
            (RoadOrient::Ns, 'w') => mi.side == Side::Left,
            (RoadOrient::Ew, 'n') => mi.side == Side::Left,
            (RoadOrient::Ew, 's') => mi.side == Side::Right,
            _ => true,
        }
    };

    let sidewalk_edge = |c: &mut Canvas, a: P, b: P, inward: P, sh_a: bool, sh_b: bool| {
        let shorten = sw * 0.707;
        let (dx, dy) = norm(b.0 - a.0, b.1 - a.1);
        let a2 = if sh_a { (a.0 + dx * shorten, a.1 + dy * shorten) } else { a };
        let b2 = if sh_b { (b.0 - dx * shorten, b.1 - dy * shorten) } else { b };
        c.line(a2, b2, 1.5, CURB);
        c.fill_poly(&[a2, b2, (b2.0 + inward.0 * sw, b2.1 + inward.1 * sw), (a2.0 + inward.0 * sw, a2.1 + inward.1 * sw)], SIDEWALK);
    };
    if show_sidewalks {
        if !north && outer('n') {
            sidewalk_edge(c, left, top, INWARD_N, !west && outer('w'), !east && outer('e'));
        }
        if !east && outer('e') {
            sidewalk_edge(c, top, right, INWARD_E, !north && outer('n'), !south && outer('s'));
        }
        if !south && outer('s') {
            sidewalk_edge(c, right, bottom, INWARD_S, !east && outer('e'), !west && outer('w'));
        }
        if !west && outer('w') {
            sidewalk_edge(c, bottom, left, INWARD_W, !south && outer('s'), !north && outer('n'));
        }
        let corner = |c: &mut Canvas, p: P, o1: P, i1: P, o2: P, i2: P| {
            let a = shortened_inner(p, o1, i1, sw);
            let b = shortened_inner(p, o2, i2, sw);
            c.fill_poly(&[p, a, b], SIDEWALK);
        };
        if !north && !east && outer('n') && outer('e') {
            corner(c, top, left, INWARD_N, right, INWARD_E);
        }
        if !east && !south && outer('e') && outer('s') {
            corner(c, right, top, INWARD_E, bottom, INWARD_S);
        }
        if !south && !west && outer('s') && outer('w') {
            corner(c, bottom, right, INWARD_S, left, INWARD_W);
        }
        if !west && !north && outer('w') && outer('n') {
            corner(c, left, bottom, INWARD_W, top, INWARD_N);
        }
    }

    // Road surface
    let surface = match mi.kind {
        RoadKind::Highway => Rgba::hex(0x3d3d3d),
        RoadKind::Avenue => Rgba::hex(0x454545),
        RoadKind::Single => ASPHALT,
    };
    let seg = |c: &mut Canvas, edge: P, d: P| {
        let stop = (cx + (edge.0 - cx) * edge_stop, cy + (edge.1 - cy) * edge_stop);
        let (px, py) = (-d.1, d.0);
        let hw = road_w * 0.5;
        c.fill_poly(
            &[
                (cx + px * hw, cy + py * hw),
                (stop.0 + px * hw, stop.1 + py * hw),
                (stop.0 - px * hw, stop.1 - py * hw),
                (cx - px * hw, cy - py * hw),
            ],
            surface,
        );
    };
    if north {
        seg(c, ne, nd);
    }
    if east {
        seg(c, ee, ed);
    }
    if south {
        seg(c, se, sd);
    }
    if west {
        seg(c, we, wd);
    }
    let cs = road_w * 1.4;
    c.fill_poly(&[(cx, cy - cs), (cx + cs, cy), (cx, cy + cs), (cx - cs, cy)], surface);

    if show_sidewalks {
        let cs = sw * 0.8;
        let four = north && east && south && west;
        if north && east {
            c.fill_poly(&[top, (top.0 - cs, top.1 + cs * 0.5), (top.0, top.1 + cs), (top.0 + cs, top.1 + cs * 0.5)], SIDEWALK);
        }
        if east && south {
            let pts = if four {
                [right, (right.0 - cs * 0.625, right.1 - cs * 1.25), (right.0 - cs * 1.25, right.1), (right.0 - cs * 0.625, right.1 + cs * 1.25)]
            } else {
                [right, (right.0 - cs, right.1 - cs * 0.5), (right.0 - cs * 2.0, right.1), (right.0 - cs, right.1 + cs * 0.5)]
            };
            c.fill_poly(&pts, SIDEWALK);
        }
        if south && west {
            c.fill_poly(&[bottom, (bottom.0 + cs, bottom.1 - cs * 0.5), (bottom.0, bottom.1 - cs), (bottom.0 - cs, bottom.1 - cs * 0.5)], SIDEWALK);
        }
        if west && north {
            let pts = if four {
                [left, (left.0 + cs * 0.625, left.1 - cs * 1.25), (left.0 + cs * 1.25, left.1), (left.0 + cs * 0.625, left.1 + cs * 1.25)]
            } else {
                [left, (left.0 + cs, left.1 - cs * 0.5), (left.0 + cs * 2.0, left.1), (left.0 + cs, left.1 + cs * 0.5)]
            };
            c.fill_poly(&pts, SIDEWALK);
        }
    }

    if pc.zoom < 0.5 {
        return;
    }
    let connections = north as u8 + east as u8 + south as u8 + west as u8;
    let intersection = connections >= 3;

    if mi.kind != RoadKind::Single && mi.side == Side::Center {
        if mi.orient == RoadOrient::Ns {
            if north {
                c.dashed_line((cx, cy), ne, 0.6, LANE_MARKING, &[2.0, 3.0], LineCap::Butt);
            }
            if south {
                c.dashed_line((cx, cy), se, 0.6, LANE_MARKING, &[2.0, 3.0], LineCap::Butt);
            }
        } else if mi.orient == RoadOrient::Ew {
            if east {
                c.dashed_line((cx, cy), ee, 0.6, LANE_MARKING, &[2.0, 3.0], LineCap::Butt);
            }
            if west {
                c.dashed_line((cx, cy), we, 0.6, LANE_MARKING, &[2.0, 3.0], LineCap::Butt);
            }
        }
    }

    if mi.has_median && mi.width >= 2 && mi.position == mi.width / 2 - 1 {
        if mi.orient == RoadOrient::Ns {
            if mi.plants && pc.zoom >= 0.55 {
                c.fill_rect(we.0 - 3.0, we.1 - 2.0, 6.0, (se.1 - we.1) + 4.0, Rgba::hex(0x6b7280));
                let n_plants = ((se.1 - we.1).abs() / 10.0).floor() as i32;
                for i in 1..n_plants {
                    let t = i as f32 / n_plants as f32;
                    c.fill_circle(we.0 + (se.0 - we.0) * t, we.1 + (se.1 - we.1) * t - 1.0, 2.0, Rgba::hex(0x4a7c3f));
                }
            } else {
                c.line((ne.0 - 1.5, ne.1), (se.0 - 1.5, se.1), 1.2, CENTER_LINE);
                c.line((ne.0 + 1.5, ne.1), (se.0 + 1.5, se.1), 1.2, CENTER_LINE);
            }
        } else if mi.orient == RoadOrient::Ew {
            if mi.plants && pc.zoom >= 0.55 {
                c.fill_rect(ee.0 - 2.0, ee.1 - 3.0, (we.0 - ee.0) + 4.0, 6.0, Rgba::hex(0x6b7280));
                let n_plants = ((we.0 - ee.0).abs() / 10.0).floor() as i32;
                for i in 1..n_plants {
                    let t = i as f32 / n_plants as f32;
                    c.fill_circle(ee.0 + (we.0 - ee.0) * t, ee.1 + (we.1 - ee.1) * t - 1.0, 2.0, Rgba::hex(0x4a7c3f));
                }
            } else {
                c.line((ee.0, ee.1 - 1.5), (we.0, we.1 - 1.5), 1.2, CENTER_LINE);
                c.line((ee.0, ee.1 + 1.5), (we.0, we.1 + 1.5), 1.2, CENTER_LINE);
            }
        }
    }

    if !intersection {
        let overlap = 8.0;
        let stop_frac = 0.58;
        let center_dash = |c: &mut Canvas, has: bool, edge: P, d: P, adj: (i32, i32)| {
            if !has {
                return;
            }
            let end = if m.is_intersection(adj.0, adj.1) {
                (cx + (edge.0 - cx) * stop_frac, cy + (edge.1 - cy) * stop_frac)
            } else {
                (edge.0 + d.0 * overlap, edge.1 + d.1 * overlap)
            };
            c.dashed_line((cx, cy), end, 0.8, CENTER_LINE, &[1.5, 2.0], LineCap::Round);
        };
        center_dash(c, north, ne, nd, (gx - 1, gy));
        center_dash(c, east, ee, ed, (gx, gy - 1));
        center_dash(c, south, se, sd, (gx + 1, gy));
        center_dash(c, west, we, wd, (gx, gy + 1));
    }

    // Direction arrows on the outer lanes of avenues/highways.
    if mi.kind != RoadKind::Single && pc.zoom >= 0.8 && mi.side != Side::Center {
        let dir = match (mi.orient, mi.side) {
            (RoadOrient::Ns, Side::Right) => Some(-std::f32::consts::PI * 0.75),
            (RoadOrient::Ns, Side::Left) => Some(std::f32::consts::PI * 0.25),
            (RoadOrient::Ew, Side::Right) => Some(-std::f32::consts::PI * 0.25),
            (RoadOrient::Ew, Side::Left) => Some(std::f32::consts::PI * 0.75),
            _ => None,
        };
        if let Some(a) = dir {
            let (sa, ca) = a.sin_cos();
            let rot = |px: f32, py: f32| (cx + px * ca - py * sa, cy + px * sa + py * ca);
            let pts = [rot(0.0, -5.0), rot(3.0, 0.0), rot(1.0, 0.0), rot(1.0, 5.0), rot(-1.0, 5.0), rot(-1.0, 0.0), rot(-3.0, 0.0)];
            c.fill_poly(&pts, Rgba::hexa(0xffffff, 0.6));
        }
    }

    // Crosswalks next to intersections.
    if pc.zoom >= 0.75 {
        let nw = norm(top.0 - left.0, top.1 - left.1);
        let nee = norm(right.0 - top.0, right.1 - top.1);
        let stripe_len = road_w * 0.22;
        let spacing = road_w * 0.30;
        let crosswalk = |c: &mut Canvas, edge: P, d: P, perp: P| {
            let cw = (cx + (edge.0 - cx) * 0.85, cy + (edge.1 - cy) * 0.85);
            for i in 0..10 {
                let off = (i as f32 - 4.5) * spacing;
                let p = (cw.0 + perp.0 * off, cw.1 + perp.1 * off);
                c.line(
                    (p.0 - d.0 * stripe_len, p.1 - d.1 * stripe_len),
                    (p.0 + d.0 * stripe_len, p.1 + d.1 * stripe_len),
                    0.7,
                    Rgba::hexa(0xffffff, 0.5),
                );
            }
        };
        if north && m.is_intersection(gx - 1, gy) {
            crosswalk(c, ne, nd, nw);
        }
        if east && m.is_intersection(gx, gy - 1) {
            crosswalk(c, ee, ed, nee);
        }
        if south && m.is_intersection(gx + 1, gy) {
            crosswalk(c, se, sd, nw);
        }
        if west && m.is_intersection(gx, gy + 1) {
            crosswalk(c, we, wd, nee);
        }
    }
}
