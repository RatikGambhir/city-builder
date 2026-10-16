//! Airplanes (sprite sheet, with contrails and shadows), helicopters that
//! shuttle between heliports, and seaplanes operating from large lakes.

use super::bake::{BakeCache, Shape};
use super::paths;
use crate::iso::{grid_to_world, tile_center, TILE_H, TILE_W};
use crate::render::shapes::{self, rgba};
use crate::render::world::View;
use crate::sprites::config::Sheet;
use crate::sprites::SheetStore;
use gpui::{Corners, RenderImage, Window};
use image::{Frame, ImageBuffer};
use isocity_core::{BuildingType as B, GameState, Rng};
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

    fn scale(self) -> f32 {
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

const PLANE_TYPES: [PlaneType; 7] = [
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

    fn paint(&mut self, window: &mut Window, v: &View, store: &SheetStore, t: PlaneType, x: f32, y: f32, angle: f32, scale: f32) {
        if let Some(s) = self.get(store, t, angle) {
            let half = s.half * scale;
            let (sx, sy) = v.to_screen(x - half, y - half);
            let size = half * 2.0 * v.cam.zoom;
            let _ = window.paint_image(v.bounds(sx, sy, size, size), Corners::default(), s.image.clone(), 0, false);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlaneState {
    TakingOff,
    Flying,
    Landing,
}

pub struct Airplane {
    x: f32,
    y: f32,
    angle: f32,
    state: PlaneState,
    speed: f32,
    altitude: f32,
    airport: (f32, f32),
    contrail: Vec<(f32, f32, f32)>,
    contrail_timer: f32,
    life: f32,
    kind: PlaneType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HeliState {
    TakingOff,
    Flying,
    Landing,
}

pub struct Helicopter {
    x: f32,
    y: f32,
    angle: f32,
    state: HeliState,
    altitude: f32,
    dest: (f32, f32),
    rotor: f32,
    color: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SeaState {
    Taxi,
    TakeOff,
    Flying,
    Landing,
}

pub struct Seaplane {
    x: f32,
    y: f32,
    angle: f32,
    target_angle: f32,
    state: SeaState,
    altitude: f32,
    speed: f32,
    timer: f32,
    bay: (f32, f32),
    bay_tiles: Arc<Vec<(i32, i32)>>,
    wake: Vec<(f32, f32, f32)>,
    flights: u32,
}

#[derive(Default)]
pub struct Aircraft {
    pub planes: Vec<Airplane>,
    pub helis: Vec<Helicopter>,
    pub seaplanes: Vec<Seaplane>,
    sprites: PlaneSprites,
    airports: Vec<(f32, f32)>,
    heliports: Vec<(f32, f32)>,
    bays: Vec<Arc<Vec<(i32, i32)>>>,
    cached_version: u64,
    plane_timer: f32,
    heli_timer: f32,
    sea_timer: f32,
}

fn mult(speed: u8) -> f32 {
    match speed {
        0 => 0.0,
        1 => 1.0,
        2 => 1.5,
        _ => 2.0,
    }
}

impl Aircraft {
    fn refresh(&mut self, state: &GameState) {
        if self.cached_version == state.game_version && self.cached_version != 0 {
            return;
        }
        self.cached_version = state.game_version.max(1);
        let g = &state.grid;
        self.airports.clear();
        self.heliports.clear();
        for t in g.iter() {
            if t.building.under_construction() || t.building.abandoned {
                continue;
            }
            let (sx, sy) = grid_to_world(t.x as f32, t.y as f32);
            match t.building.kind {
                B::Airport => {
                    self.airports.push((sx + TILE_W * 0.5 + 0.0, sy + TILE_H * 2.0));
                    self.heliports.push((sx + TILE_W * 0.5, sy + TILE_H * 2.0));
                }
                B::Hospital | B::PoliceStation => {
                    let fp = t.building.kind.size();
                    let (cx, cy) = tile_center(t.x as f32 + (fp.width - 1) as f32 / 2.0, t.y as f32 + (fp.height - 1) as f32 / 2.0);
                    self.heliports.push((cx, cy - 10.0));
                }
                _ => {}
            }
        }
        // Large lakes/bays for seaplanes.
        self.bays.clear();
        let mut seen = std::collections::HashSet::new();
        for wb in &state.water_bodies {
            let tiles: Vec<(i32, i32)> = wb.tiles.iter().map(|p| (p.x, p.y)).filter(|&(x, y)| g.is_water(x, y)).collect();
            if tiles.len() >= 12 && seen.insert(wb.id.clone()) {
                self.bays.push(Arc::new(tiles));
            }
        }
        if self.bays.is_empty() {
            // Imported cities may lack water body metadata: find a big lake.
            for t in g.iter() {
                if g.is_water(t.x, t.y) && paths::water_body_size(g, t.x, t.y, 40) >= 12 {
                    let tiles: Vec<(i32, i32)> = g
                        .iter()
                        .filter(|o| g.is_water(o.x, o.y) && (o.x - t.x).abs() < 8 && (o.y - t.y).abs() < 8)
                        .map(|o| (o.x, o.y))
                        .collect();
                    self.bays.push(Arc::new(tiles));
                    break;
                }
            }
        }
    }

    pub fn update(&mut self, dt: f32, state: &GameState, rng: &mut Rng) {
        self.refresh(state);
        let m = mult(state.speed);
        if m == 0.0 {
            return;
        }
        let pop = state.stats.population;
        let n = state.grid.size() as f32;

        // Airplanes
        if self.airports.is_empty() || pop < 2000 {
            self.planes.clear();
        } else {
            let max = ((pop / 2000) as usize * 3).clamp(25, 80);
            self.plane_timer -= dt;
            if self.planes.len() < max && self.plane_timer <= 0.0 {
                self.plane_timer = 2.0 + rng.next_f32() * 5.0;
                let airport = *rng.pick(&self.airports).unwrap();
                let kind = *rng.pick(&PLANE_TYPES).unwrap();
                if rng.chance(0.5) {
                    self.planes.push(Airplane {
                        x: airport.0,
                        y: airport.1,
                        angle: rng.next_f32() * TAU,
                        state: PlaneState::TakingOff,
                        speed: 30.0 + rng.next_f32() * 20.0,
                        altitude: 0.0,
                        airport,
                        contrail: Vec::new(),
                        contrail_timer: 0.0,
                        life: 30.0 + rng.next_f32() * 20.0,
                        kind,
                    });
                } else {
                    let extent = n * TILE_W;
                    let (cx, cy) = (0.0, n * TILE_H / 2.0);
                    let (sx, sy) = match rng.below(4) {
                        0 => (cx + (rng.next_f32() - 0.5) * extent, cy - extent / 2.0 - 200.0),
                        1 => (cx + extent / 2.0 + 200.0, cy + (rng.next_f32() - 0.5) * extent / 2.0),
                        2 => (cx + (rng.next_f32() - 0.5) * extent, cy + extent / 2.0 + 200.0),
                        _ => (cx - extent / 2.0 - 200.0, cy + (rng.next_f32() - 0.5) * extent / 2.0),
                    };
                    self.planes.push(Airplane {
                        x: sx,
                        y: sy,
                        angle: (airport.1 - sy).atan2(airport.0 - sx),
                        state: PlaneState::Flying,
                        speed: 80.0 + rng.next_f32() * 40.0,
                        altitude: 1.0,
                        airport,
                        contrail: Vec::new(),
                        contrail_timer: 0.0,
                        life: 30.0 + rng.next_f32() * 20.0,
                        kind,
                    });
                }
            }
        }
        self.planes.retain_mut(|p| {
            for c in &mut p.contrail {
                c.2 += dt;
            }
            p.contrail.retain(|c| c.2 < 3.0);
            if p.altitude > 0.7 {
                p.contrail_timer += dt;
                while p.contrail_timer >= 0.02 {
                    p.contrail_timer -= 0.02;
                    p.contrail.push((p.x - p.angle.cos() * 40.0 * p.kind.scale() / 0.152, p.y - p.angle.sin() * 40.0 * p.kind.scale() / 0.152 + 8.0, 0.0));
                }
            }
            let step = p.speed * dt * m;
            match p.state {
                PlaneState::TakingOff => {
                    p.x += p.angle.cos() * step;
                    p.y += p.angle.sin() * step;
                    p.altitude = (p.altitude + dt * 0.3).min(1.0);
                    p.speed = (p.speed + dt * 20.0).min(120.0);
                    if p.altitude >= 1.0 {
                        p.state = PlaneState::Flying;
                    }
                }
                PlaneState::Flying => {
                    p.x += p.angle.cos() * step;
                    p.y += p.angle.sin() * step;
                    p.life -= dt;
                    let d = ((p.x - p.airport.0).powi(2) + (p.y - p.airport.1).powi(2)).sqrt();
                    if d < 400.0 && p.life < 10.0 {
                        p.state = PlaneState::Landing;
                    } else if p.life <= 0.0 {
                        return false;
                    }
                }
                PlaneState::Landing => {
                    p.angle = (p.airport.1 - p.y).atan2(p.airport.0 - p.x);
                    p.x += p.angle.cos() * step;
                    p.y += p.angle.sin() * step;
                    p.altitude = (p.altitude - dt * 0.25).max(0.0);
                    p.speed = (p.speed - dt * 15.0).max(30.0);
                    let d = ((p.x - p.airport.0).powi(2) + (p.y - p.airport.1).powi(2)).sqrt();
                    if d < 50.0 || p.altitude <= 0.0 {
                        return false;
                    }
                }
            }
            true
        });

        // Helicopters
        if self.heliports.len() < 2 || pop < 3000 {
            self.helis.clear();
        } else {
            let max = (self.heliports.len() * 2).clamp(2, 12);
            self.heli_timer -= dt;
            if self.helis.len() < max && self.heli_timer <= 0.0 {
                self.heli_timer = 3.0 + rng.next_f32() * 4.0;
                let a = *rng.pick(&self.heliports).unwrap();
                let b = *rng.pick(&self.heliports).unwrap();
                if a != b {
                    self.helis.push(Helicopter {
                        x: a.0,
                        y: a.1,
                        angle: (b.1 - a.1).atan2(b.0 - a.0),
                        state: HeliState::TakingOff,
                        altitude: 0.0,
                        dest: b,
                        rotor: 0.0,
                        color: [0xdc2626, 0xffffff, 0x1e3a8a, 0xf97316, 0x059669][rng.below(5)],
                    });
                }
            }
        }
        self.helis.retain_mut(|h| {
            h.rotor += dt * 30.0;
            match h.state {
                HeliState::TakingOff => {
                    h.altitude = (h.altitude + dt * 0.5).min(0.5);
                    if h.altitude >= 0.5 {
                        h.state = HeliState::Flying;
                    }
                }
                HeliState::Flying => {
                    let (dx, dy) = (h.dest.0 - h.x, h.dest.1 - h.y);
                    let d = (dx * dx + dy * dy).sqrt();
                    h.angle = dy.atan2(dx);
                    let step = 60.0 * dt * m;
                    if d <= step + 5.0 {
                        h.state = HeliState::Landing;
                    } else {
                        h.x += dx / d * step;
                        h.y += dy / d * step;
                    }
                }
                HeliState::Landing => {
                    h.altitude -= dt * 0.4;
                    if h.altitude <= 0.0 {
                        return false;
                    }
                }
            }
            true
        });

        // Seaplanes
        if self.bays.is_empty() || pop < 3000 {
            self.seaplanes.clear();
        } else {
            self.sea_timer -= dt;
            if self.seaplanes.len() < (self.bays.len() * 2).min(25) && self.sea_timer <= 0.0 {
                self.sea_timer = 4.0 + rng.next_f32() * 6.0;
                let bay = rng.pick(&self.bays).unwrap().clone();
                let &(tx, ty) = rng.pick(&bay).unwrap();
                let (x, y) = tile_center(tx as f32, ty as f32);
                let a = rng.next_f32() * TAU;
                self.seaplanes.push(Seaplane {
                    x,
                    y,
                    angle: a,
                    target_angle: a,
                    state: SeaState::Taxi,
                    altitude: 0.0,
                    speed: 18.0,
                    timer: 4.0 + rng.next_f32() * 6.0,
                    bay: (x, y),
                    bay_tiles: bay,
                    wake: Vec::new(),
                    flights: 0,
                });
            }
        }
        let grid = &state.grid;
        self.seaplanes.retain_mut(|s| {
            for w in &mut s.wake {
                w.2 += dt;
            }
            s.wake.retain(|w| w.2 < 2.0);
            let on_water = |x: f32, y: f32| {
                let (gx, gy) = crate::iso::world_to_grid(x, y);
                grid.is_water(gx, gy)
            };
            match s.state {
                SeaState::Taxi => {
                    s.timer -= dt * m;
                    // Wander, steering back toward the bay if leaving water.
                    let ahead = (s.x + s.angle.cos() * 20.0, s.y + s.angle.sin() * 20.0);
                    if !on_water(ahead.0, ahead.1) {
                        s.target_angle = (s.bay.1 - s.y).atan2(s.bay.0 - s.x);
                    } else if rng.chance(0.01) {
                        s.target_angle += (rng.next_f32() - 0.5) * 1.5;
                    }
                    let mut d = s.target_angle - s.angle;
                    while d > PI {
                        d -= TAU;
                    }
                    while d < -PI {
                        d += TAU;
                    }
                    s.angle += d.clamp(-dt * 1.5, dt * 1.5);
                    s.x += s.angle.cos() * s.speed * dt * m;
                    s.y += s.angle.sin() * s.speed * dt * m;
                    s.wake.push((s.x, s.y, 0.0));
                    if s.timer <= 0.0 {
                        if s.flights >= 3 {
                            return false;
                        }
                        s.state = SeaState::TakeOff;
                        s.speed = 30.0;
                    }
                }
                SeaState::TakeOff => {
                    s.speed = (s.speed + dt * 30.0).min(90.0);
                    s.x += s.angle.cos() * s.speed * dt * m;
                    s.y += s.angle.sin() * s.speed * dt * m;
                    if s.speed > 55.0 {
                        s.altitude = (s.altitude + dt * 0.35).min(1.0);
                    } else {
                        s.wake.push((s.x, s.y, 0.0));
                    }
                    if s.altitude >= 1.0 {
                        s.state = SeaState::Flying;
                        s.timer = 25.0 + rng.next_f32() * 25.0;
                        s.flights += 1;
                    }
                }
                SeaState::Flying => {
                    s.timer -= dt * m;
                    if s.timer < 10.0 {
                        s.target_angle = (s.bay.1 - s.y).atan2(s.bay.0 - s.x);
                    } else if rng.chance(0.005) {
                        s.target_angle += (rng.next_f32() - 0.5) * 2.0;
                    }
                    let mut d = s.target_angle - s.angle;
                    while d > PI {
                        d -= TAU;
                    }
                    while d < -PI {
                        d += TAU;
                    }
                    s.angle += d.clamp(-dt * 0.8, dt * 0.8);
                    s.x += s.angle.cos() * s.speed * dt * m;
                    s.y += s.angle.sin() * s.speed * dt * m;
                    let dist = ((s.x - s.bay.0).powi(2) + (s.y - s.bay.1).powi(2)).sqrt();
                    if s.timer < 10.0 && dist < 150.0 {
                        s.state = SeaState::Landing;
                    }
                }
                SeaState::Landing => {
                    s.angle = (s.bay.1 - s.y).atan2(s.bay.0 - s.x);
                    s.speed = (s.speed - dt * 10.0).max(25.0);
                    s.x += s.angle.cos() * s.speed * dt * m;
                    s.y += s.angle.sin() * s.speed * dt * m;
                    s.altitude = (s.altitude - dt * 0.35).max(0.0);
                    if s.altitude <= 0.0 {
                        s.state = SeaState::Taxi;
                        s.speed = 18.0;
                        s.timer = 4.0 + rng.next_f32() * 6.0;
                        let &(tx, ty) = rng.pick(&s.bay_tiles).unwrap();
                        s.bay = tile_center(tx as f32, ty as f32);
                    }
                }
            }
            true
        });
    }

    /// Wakes and taxiing seaplanes (water level).
    pub fn paint_water_level(&mut self, window: &mut Window, v: &View, store: &SheetStore) {
        let z = v.cam.zoom;
        for s in &self.seaplanes {
            for &(x, y, age) in &s.wake {
                let (sx, sy) = v.to_screen(x, y);
                shapes::circle(window, sx, sy, (1.0 + age * 2.0) * z, rgba(0xffffff, (0.35 - age * 0.17).max(0.0)));
            }
        }
        let taxiing: Vec<(f32, f32, f32)> = self.seaplanes.iter().filter(|s| s.altitude < 0.05).map(|s| (s.x, s.y, s.angle)).collect();
        for (x, y, a) in taxiing {
            self.sprites.paint(window, v, store, PlaneType::Seaplane, x, y, a, 0.7);
        }
    }

    pub fn paint_air(&mut self, window: &mut Window, v: &View, store: &SheetStore, bake: &mut BakeCache) {
        let z = v.cam.zoom;
        let (vx, vy, vw, vh) = v.world_rect();
        let vis = |x: f32, y: f32, m: f32| x > vx - m && x < vx + vw + m && y > vy - m && y < vy + vh + m;
        // Contrails first, behind every plane.
        for p in &self.planes {
            for &(x, y, age) in &p.contrail {
                if !vis(x, y, 20.0) {
                    continue;
                }
                let op = (1.0 - age / 3.0).max(0.0) * 0.4 * p.altitude;
                let (sx, sy) = v.to_screen(x, y);
                shapes::circle(window, sx, sy, (3.0 + age * 8.0) * z, rgba(0xffffff, op));
            }
        }
        let planes: Vec<(PlaneType, f32, f32, f32, f32)> = self.planes.iter().map(|p| (p.kind, p.x, p.y, p.angle, p.altitude)).collect();
        for (kind, x, y, a, alt) in planes {
            if !vis(x, y, 80.0) {
                continue;
            }
            if alt < 0.8 {
                let off = (1.0 - alt) * 18.0;
                let (sx, sy) = v.to_screen(x + off, y + off * 0.5);
                let s = (0.55 + alt * 0.35) * kind.scale() / 0.152;
                shapes::ellipse(window, sx, sy, 50.0 * 0.152 * s * z, 20.0 * 0.152 * s * 0.5 * z, rgba(0x000000, 0.25 * (1.0 - alt)));
            }
            self.sprites.paint(window, v, store, kind, x, y, a, 0.7 + alt * 0.5);
        }
        for h in &self.helis {
            if !vis(h.x, h.y, 40.0) {
                continue;
            }
            let lift = h.altitude * 40.0;
            let (shx, shy) = v.to_screen(h.x + lift * 0.4, h.y + lift * 0.2);
            shapes::ellipse(window, shx, shy, 7.0 * z, 3.0 * z, rgba(0x000000, 0.2));
            let y = h.y - lift;
            bake.paint(window, v, Shape::Helicopter(h.color), h.x, y, h.angle);
            let (sx, sy) = v.to_screen(h.x, y);
            let r = 11.0 * z;
            for k in 0..2 {
                let a = h.rotor + k as f32 * PI / 2.0;
                let (dx, dy) = (a.cos() * r, a.sin() * r * 0.5);
                shapes::line(window, (sx - dx, sy - dy), (sx + dx, sy + dy), 1.2 * z, rgba(0x1f2937, 0.7));
            }
        }
        let flying: Vec<(f32, f32, f32, f32)> = self.seaplanes.iter().filter(|s| s.altitude >= 0.05).map(|s| (s.x, s.y, s.angle, s.altitude)).collect();
        for (x, y, a, alt) in flying {
            if vis(x, y, 60.0) {
                self.sprites.paint(window, v, store, PlaneType::Seaplane, x, y, a, 0.7 + alt * 0.5);
            }
        }
    }
}
