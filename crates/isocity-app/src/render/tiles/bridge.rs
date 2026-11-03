//! Road and rail bridges over water.

use crate::render::canvas2d::{Canvas, Rgba, P};
use crate::iso::{grid_to_world, TILE_H, TILE_W};
use isocity_core::{BridgeType, Orientation};
use tiny_skia::LineCap;
use super::{PaintCtx, norm};
use super::ground::draw_water_under;
use super::palette::{BRIDGE_DECK_RAIL, CENTER_LINE, RAIL};
use super::rail::offset;

// ---------------------------------------------------------------------------
// Bridges
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct BridgeStyle {
    asphalt: u32,
    barrier: u32,
    support: u32,
    cable: Option<u32>,
}

fn bridge_style(kind: BridgeType, variant: u8) -> BridgeStyle {
    let v = variant as usize;
    let s = |asphalt, barrier, support, cable| BridgeStyle { asphalt, barrier, support, cable };
    match kind {
        BridgeType::Small => [s(0x4a4a4a, 0x707070, 0x404040, None), s(0x454545, 0x606060, 0x353535, None), s(0x3d3d3d, 0x585858, 0x303030, None)][v.min(2)],
        BridgeType::Medium => [s(0x4a4a4a, 0x808080, 0x505050, None), s(0x454545, 0x707070, 0x454545, None), s(0x3d3d3d, 0x656565, 0x404040, None)][v.min(2)],
        BridgeType::Large => [s(0x3d3d3d, 0x4682b4, 0x3a5a5a, None), s(0x4a4a4a, 0x708090, 0x405060, None)][v.min(1)],
        BridgeType::Suspension => [
            s(0x3d3d3d, 0x707070, 0x909090, Some(0xdc143c)),
            s(0x3d3d3d, 0x606060, 0x808080, Some(0x708090)),
            s(0x3d3d3d, 0x656560, 0x858580, Some(0x5a7a5a)),
        ][v.min(2)],
    }
}

pub fn draw_bridge(c: &mut Canvas, pc: &PaintCtx, gx: i32, gy: i32) {
    let t = pc.map.get(gx, gy);
    let Some(b) = t.bridge else {
        return;
    };
    draw_water_under(c, pc, gx, gy);
    let (x, y0) = grid_to_world(gx as f32, gy as f32);
    let y = y0 + if b.rail { TILE_H * 0.1 } else { 0.0 };
    let style = bridge_style(b.kind, b.variant);
    let (w, h) = (TILE_W, TILE_H);
    let ne = (x + w * 0.25, y + h * 0.25);
    let ee = (x + w * 0.75, y + h * 0.25);
    let se = (x + w * 0.75, y + h * 0.75);
    let we = (x + w * 0.25, y + h * 0.75);
    let len = ((w / 2.0).powi(2) + (h / 2.0).powi(2)).sqrt();
    let (start, end, perp) = if b.orient == Orientation::Ns {
        (ne, se, (-(w / 2.0) / len, (h / 2.0) / len))
    } else {
        (ee, we, ((w / 2.0) / len, (h / 2.0) / len))
    };
    // Extend deck slightly past the tile edges so spans join seamlessly.
    let dir = norm(end.0 - start.0, end.1 - start.1);
    let ext = 1.0;
    let start = (start.0 - dir.0 * ext, start.1 - dir.1 * ext);
    let end = (end.0 + dir.0 * ext, end.1 + dir.1 * ext);
    let half = w * if b.rail { 0.36 } else { 0.45 } / 2.0;
    let deck_drop = 3.0;
    let support = Rgba::hex(style.support);

    // Pillars
    let pillar = |c: &mut Canvas, p: P| {
        c.fill_rect(p.0 - 1.5, p.1, 3.0, 7.0, support);
    };
    if b.index % 2 == 0 || b.span <= 2 {
        let m = ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0);
        pillar(c, (m.0 + perp.0 * half * 0.7, m.1 + perp.1 * half * 0.7 + 2.0));
        pillar(c, (m.0 - perp.0 * half * 0.7, m.1 - perp.1 * half * 0.7 + 2.0));
    }
    // Deck side (thickness)
    let deck = [
        offset(start, perp, half),
        offset(end, perp, half),
        offset(end, perp, -half),
        offset(start, perp, -half),
    ];
    let side = [deck[1], deck[2], (deck[2].0, deck[2].1 + deck_drop), (deck[1].0, deck[1].1 + deck_drop)];
    c.fill_poly(&side, support);
    let side2 = [deck[2], deck[3], (deck[3].0, deck[3].1 + deck_drop), (deck[2].0, deck[2].1 + deck_drop)];
    c.fill_poly(&side2, Rgba::hex(style.support).alpha(0.85));
    // Deck surface
    let deck_color = if b.rail { BRIDGE_DECK_RAIL } else { Rgba::hex(style.asphalt) };
    c.fill_poly(&deck, deck_color);
    if b.rail {
        let half_sep = TILE_W * 0.22 / 2.0;
        let half_gauge = TILE_W * 0.06 / 2.0;
        for side in [half_sep, -half_sep] {
            for i in 0..7 {
                let tt = (i as f32 + 0.5) / 7.0;
                let p = (start.0 + (end.0 - start.0) * tt, start.1 + (end.1 - start.1) * tt);
                let p = offset(p, perp, side);
                c.line(offset(p, perp, half_gauge * 1.6), offset(p, perp, -half_gauge * 1.6), 1.0, Rgba::hex(0x4a4a4a));
            }
            for g in [half_gauge, -half_gauge] {
                c.line(offset(start, perp, side + g), offset(end, perp, side + g), 0.85, RAIL);
            }
        }
    } else if pc.zoom >= 0.5 {
        c.dashed_line(start, end, 0.8, CENTER_LINE, &[1.5, 2.0], LineCap::Round);
    }
    // Railings
    let barrier = Rgba::hex(style.barrier);
    for sgn in [1.0, -1.0] {
        let a = offset(start, perp, half * sgn);
        let bpt = offset(end, perp, half * sgn);
        c.line((a.0, a.1 - 1.5), (bpt.0, bpt.1 - 1.5), 1.2, barrier);
        if pc.zoom >= 0.6 {
            for i in 0..=3 {
                let tt = i as f32 / 3.0;
                let p = (a.0 + (bpt.0 - a.0) * tt, a.1 + (bpt.1 - a.1) * tt);
                c.line(p, (p.0, p.1 - 1.5), 0.8, barrier);
            }
        }
    }
    // Trusses for large bridges
    if b.kind == BridgeType::Large && pc.zoom >= 0.5 {
        let accent = Rgba::hex(style.barrier);
        for sgn in [1.0, -1.0] {
            let a = offset(start, perp, half * sgn);
            let bpt = offset(end, perp, half * sgn);
            let top_a = (a.0, a.1 - 6.0);
            let top_b = (bpt.0, bpt.1 - 6.0);
            c.line(top_a, top_b, 1.0, accent);
            c.line(a, top_b, 0.8, accent);
            c.line(top_a, bpt, 0.8, accent);
        }
    }
    // Suspension towers and cables
    if b.kind == BridgeType::Suspension {
        let cable = Rgba::hex(style.cable.unwrap_or(0x707070));
        let tower_at = (b.span / 2).saturating_sub(1).max(1);
        let span = b.span.max(2) as f32;
        let tower_h = 26.0;
        for sgn in [1.0, -1.0] {
            let a = offset(start, perp, half * sgn);
            let bpt = offset(end, perp, half * sgn);
            // Cable height follows a parabola over the whole span.
            let hfun = |i: f32| {
                let u = i / span;
                let tower_u = (tower_at as f32 + 0.5) / span;
                let du = ((u - tower_u).abs() / tower_u.max(1.0 - tower_u)).min(1.0);
                6.0 + (tower_h - 6.0) * du * du
            };
            let ha = hfun(b.index as f32);
            let hb = hfun(b.index as f32 + 1.0);
            c.line((a.0, a.1 - ha), (bpt.0, bpt.1 - hb), 1.2, cable);
            if pc.zoom >= 0.6 {
                for i in 1..4 {
                    let tt = i as f32 / 4.0;
                    let p = (a.0 + (bpt.0 - a.0) * tt, a.1 + (bpt.1 - a.1) * tt);
                    let hh = ha + (hb - ha) * tt;
                    c.line(p, (p.0, p.1 - hh), 0.5, cable.alpha(0.8));
                }
            }
            if b.index == tower_at || b.index == b.span.saturating_sub(1 + tower_at) {
                let m = ((a.0 + bpt.0) / 2.0, (a.1 + bpt.1) / 2.0);
                c.fill_rect(m.0 - 1.5, m.1 - tower_h - 2.0, 3.0, tower_h + 2.0, support);
                c.fill_rect(m.0 - 2.0, m.1 - tower_h - 3.0, 4.0, 2.0, Rgba::hex(style.barrier));
            }
        }
    }
}
