//! Static ground layer painting (everything the web version draws on its main
//! canvas): base tiles, zone tints, water with beaches, roads with markings,
//! rail tracks, bridges, building bases and construction plots.

use super::canvas2d::{Canvas, Rgba, P};
use crate::iso::{grid_to_world, TILE_H, TILE_W};
use isocity_core::{
    sim::find_building_origin, BridgePosition, BridgeType, BuildingType as B, GameState, Orientation, TrackType,
    ZoneType,
};
use std::sync::Arc;
use tiny_skia::{LineCap, Pixmap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BridgeInfo {
    pub kind: BridgeType,
    pub orient: Orientation,
    pub variant: u8,
    pub pos: BridgePosition,
    pub index: u8,
    pub span: u8,
    pub rail: bool,
}

/// Everything about a tile that affects the static ground layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileInfo {
    pub in_bounds: bool,
    pub kind: B,
    pub zone: ZoneType,
    pub rail_overlay: bool,
    pub park: bool,
    pub grey_base: bool,
    pub foundation: bool,
    pub station: bool,
    pub marina_pier: bool,
    pub bridge: Option<BridgeInfo>,
}

impl Default for TileInfo {
    fn default() -> Self {
        Self {
            in_bounds: false,
            kind: B::Grass,
            zone: ZoneType::None,
            rail_overlay: false,
            park: false,
            grey_base: false,
            foundation: false,
            station: false,
            marina_pier: false,
            bridge: None,
        }
    }
}

/// Precomputed [`TileInfo`] for a whole grid.
#[derive(Clone)]
pub struct TileMap {
    pub size: i32,
    pub tiles: Arc<Vec<TileInfo>>,
}

impl TileMap {
    pub fn build(state: &GameState) -> Self {
        let grid = &state.grid;
        let size = grid.size();
        let mut tiles = vec![TileInfo::default(); (size * size) as usize];
        for t in grid.iter() {
            let b = &t.building;
            let (park, part_of_building, origin_kind, origin_foundation) = if b.kind == B::Empty {
                match find_building_origin(grid, t.x, t.y) {
                    Some((ox, oy, k)) => (
                        k.is_park(),
                        true,
                        Some(k),
                        grid.at(ox, oy).building.construction_progress < 40.0,
                    ),
                    None => (false, false, None, false),
                }
            } else {
                (b.kind.is_park(), false, Some(b.kind), b.construction_progress < 40.0 && b.kind.needs_construction())
            };
            let direct = !park && !matches!(b.kind, B::Grass | B::Empty | B::Water | B::Road | B::Bridge | B::Rail | B::Tree);
            let grey_base = (direct || (b.kind == B::Empty && part_of_building)) && !park;
            let is_building = origin_kind.map_or(false, |k| !k.is_terrain());
            tiles[(t.y * size + t.x) as usize] = TileInfo {
                in_bounds: true,
                kind: b.kind,
                zone: t.zone,
                rail_overlay: t.has_rail_overlay,
                park,
                grey_base,
                foundation: is_building && origin_foundation,
                station: origin_kind == Some(B::RailStation),
                marina_pier: matches!(origin_kind, Some(B::MarinaDocksSmall) | Some(B::PierLarge)),
                bridge: if b.kind == B::Bridge {
                    Some(BridgeInfo {
                        kind: b.bridge_type.unwrap_or(BridgeType::Large),
                        orient: b.bridge_orientation.unwrap_or(Orientation::Ns),
                        variant: b.bridge_variant.unwrap_or(0),
                        pos: b.bridge_position.unwrap_or(BridgePosition::Middle),
                        index: b.bridge_index.unwrap_or(0),
                        span: b.bridge_span.unwrap_or(1),
                        rail: b.bridge_track_type == Some(TrackType::Rail),
                    })
                } else {
                    None
                },
            };
        }
        Self {
            size,
            tiles: Arc::new(tiles),
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> TileInfo {
        if x < 0 || y < 0 || x >= self.size || y >= self.size {
            TileInfo::default()
        } else {
            self.tiles[(y * self.size + x) as usize]
        }
    }

    pub fn is_water(&self, x: i32, y: i32) -> bool {
        let t = self.get(x, y);
        t.in_bounds && t.kind == B::Water
    }

    pub fn has_road(&self, x: i32, y: i32) -> bool {
        let t = self.get(x, y);
        match t.kind {
            B::Road => t.in_bounds,
            B::Bridge => t.bridge.map_or(true, |b| !b.rail),
            _ => false,
        }
    }

    pub fn has_rail(&self, x: i32, y: i32) -> bool {
        let t = self.get(x, y);
        t.in_bounds
            && (t.kind == B::Rail
                || t.station
                || (t.kind == B::Road && t.rail_overlay)
                || t.bridge.map_or(false, |b| b.rail))
    }

    pub fn road_count(&self, x: i32, y: i32) -> u8 {
        self.has_road(x - 1, y) as u8 + self.has_road(x, y - 1) as u8 + self.has_road(x + 1, y) as u8 + self.has_road(x, y + 1) as u8
    }

    pub fn is_intersection(&self, x: i32, y: i32) -> bool {
        self.has_road(x, y) && self.road_count(x, y) >= 3
    }

    /// Hash of all tiles in a rectangle (used to detect chunk changes).
    pub fn region_hash(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for y in y0..y1 {
            for x in x0..x1 {
                self.get(x, y).hash(&mut h);
            }
        }
        h.finish()
    }
}

// ---------------------------------------------------------------------------
// Road merge analysis (avenues & highways)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoadKind {
    Single,
    Avenue,
    Highway,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RoadOrient {
    Ns,
    Ew,
    Intersection,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Left,
    Right,
    Center,
    Single,
}

#[derive(Clone, Copy, Debug)]
pub struct MergeInfo {
    pub kind: RoadKind,
    pub orient: RoadOrient,
    pub has_median: bool,
    pub plants: bool,
    pub position: i32,
    pub width: i32,
    pub side: Side,
}

pub fn analyze_merged_road(m: &TileMap, x: i32, y: i32) -> MergeInfo {
    let single = MergeInfo {
        kind: RoadKind::Single,
        orient: RoadOrient::Intersection,
        has_median: false,
        plants: false,
        position: 0,
        width: 1,
        side: Side::Single,
    };
    if !m.has_road(x, y) {
        return single;
    }
    let (n, e, s, w) = (m.has_road(x - 1, y), m.has_road(x, y - 1), m.has_road(x + 1, y), m.has_road(x, y + 1));
    let count = n as u8 + e as u8 + s as u8 + w as u8;
    if count >= 3 {
        return MergeInfo { side: Side::Center, ..single };
    }
    let ns = n || s;
    let ew = e || w;
    let build = |orient, before: i32, after: i32, first_side: Side, last_side: Side| {
        let width = before + after + 1;
        let position = before;
        let side = if width > 1 {
            if position == 0 {
                first_side
            } else if position == width - 1 {
                last_side
            } else {
                Side::Center
            }
        } else {
            Side::Single
        };
        MergeInfo {
            kind: if width >= 4 {
                RoadKind::Highway
            } else if width >= 2 {
                RoadKind::Avenue
            } else {
                RoadKind::Single
            },
            orient,
            has_median: width >= 2,
            plants: width >= 3,
            position,
            width,
            side,
        }
    };
    if ns && !ew {
        let count_dir = |dy: i32| {
            let mut c = 0;
            for d in 1..=3 {
                let yy = y + dy * d;
                if m.has_road(x, yy) && (m.has_road(x - 1, yy) || m.has_road(x + 1, yy)) {
                    c += 1;
                } else {
                    break;
                }
            }
            c
        };
        let west = count_dir(1);
        let east = count_dir(-1);
        return build(RoadOrient::Ns, east, west, Side::Right, Side::Left);
    }
    if ew && !ns {
        let count_dir = |dx: i32| {
            let mut c = 0;
            for d in 1..=3 {
                let xx = x + dx * d;
                if m.has_road(xx, y) && (m.has_road(xx, y - 1) || m.has_road(xx, y + 1)) {
                    c += 1;
                } else {
                    break;
                }
            }
            c
        };
        let north = count_dir(-1);
        let south = count_dir(1);
        return build(RoadOrient::Ew, north, south, Side::Left, Side::Right);
    }
    MergeInfo {
        orient: if count >= 2 { RoadOrient::Intersection } else { RoadOrient::Ns },
        ..single
    }
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

const ASPHALT: Rgba = Rgba::hex(0x4a4a4a);
const SIDEWALK: Rgba = Rgba::hex(0x9ca3af);
const CURB: Rgba = Rgba::hex(0x6b7280);
const LANE_MARKING: Rgba = Rgba::hex(0xffffff);
const CENTER_LINE: Rgba = Rgba::hex(0xfbbf24);
const BEACH_FILL: Rgba = Rgba::hex(0xd4a574);
const BEACH_CURB: Rgba = Rgba::hex(0xb8956a);

const BALLAST: Rgba = Rgba::hex(0x9b8365);
const TIE: Rgba = Rgba::hex(0x5c422e);
const RAIL: Rgba = Rgba::hex(0x6a6a6a);
const RAIL_SHADOW: Rgba = Rgba::hex(0x404040);
const BRIDGE_DECK_RAIL: Rgba = Rgba::hex(0x7a8088);

fn diamond(x: f32, y: f32) -> [P; 4] {
    [
        (x + TILE_W / 2.0, y),
        (x + TILE_W, y + TILE_H / 2.0),
        (x + TILE_W / 2.0, y + TILE_H),
        (x, y + TILE_H / 2.0),
    ]
}

fn norm(dx: f32, dy: f32) -> P {
    let l = (dx * dx + dy * dy).sqrt().max(1e-6);
    (dx / l, dy / l)
}

/// Painting parameters shared by a chunk bake.
pub struct PaintCtx<'a> {
    pub map: &'a TileMap,
    pub zoom: f32,
    pub water: Option<&'a Pixmap>,
}

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

const INWARD_N: P = (0.707, 0.707);
const INWARD_E: P = (-0.707, 0.707);
const INWARD_S: P = (-0.707, -0.707);
const INWARD_W: P = (0.707, -0.707);

fn shortened_inner(corner: P, other: P, inward: P, width: f32) -> P {
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
    curve_perp: P,
}

fn rail_segments(x: f32, y: f32, n: bool, e: bool, s: bool, w: bool) -> (Vec<TrackSeg>, bool) {
    let (tw, th) = (TILE_W, TILE_H);
    let c = (x + tw / 2.0, y + th / 2.0);
    let ne = (x + tw * 0.25, y + th * 0.25);
    let ee = (x + tw * 0.75, y + th * 0.25);
    let se = (x + tw * 0.75, y + th * 0.75);
    let we = (x + tw * 0.25, y + th * 0.75);
    let straight = |a: P, b: P, perp: P| TrackSeg { from: a, to: b, ctrl: None, from_perp: perp, to_perp: perp, curve_perp: perp };
    let curve = |a: P, b: P, fp: P, tp: P, cp: P| TrackSeg { from: a, to: b, ctrl: Some(c), from_perp: fp, to_perp: tp, curve_perp: cp };
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

fn offset(p: P, d: P, a: f32) -> P {
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
                let lat = if seg.ctrl.is_some() { side } else { side };
                c.fill_poly(&track_band(seg, lat, half_ballast), BALLAST);
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
