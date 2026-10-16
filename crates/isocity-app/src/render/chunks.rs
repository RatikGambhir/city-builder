//! The static ground layer is baked into textures per 8x8-tile chunk at a
//! zoom-dependent level of detail on background threads, then drawn as GPU
//! images. Chunks are re-baked when any tile they depend on changes.

use super::canvas2d::Canvas;
use super::tiles::{self, PaintCtx, TileMap};
use crate::iso::{grid_to_world, TILE_H, TILE_W};
use gpui::RenderImage;
use image::{Frame, ImageBuffer};
use isocity_core::BuildingType as B;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use tiny_skia::{Pixmap, Transform};

pub const CHUNK: i32 = 8;
const RING: i32 = 2;
const PAD: f32 = 40.0;
/// Texture memory budget in pixels (~256 MB of BGRA).
const PIXEL_BUDGET: u64 = 64_000_000;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChunkKey {
    pub cx: i32,
    pub cy: i32,
    pub lod: u8,
}

/// LOD index -> pixels per world unit.
pub fn lod_scale(lod: u8) -> f32 {
    [0.25, 0.5, 1.0, 2.0, 4.0][lod.min(4) as usize]
}

/// Picks the LOD for a zoom level and display scale factor.
pub fn lod_for(zoom: f32, scale_factor: f32) -> u8 {
    let s = zoom * scale_factor;
    if s <= 0.3 {
        0
    } else if s <= 0.6 {
        1
    } else if s <= 1.2 {
        2
    } else if s <= 2.4 {
        3
    } else {
        4
    }
}

/// World-space rectangle covered by a chunk image (including padding).
pub fn chunk_world_rect(cx: i32, cy: i32) -> (f32, f32, f32, f32) {
    let (x0, y0) = (cx * CHUNK - RING, cy * CHUNK - RING);
    let (x1, y1) = (cx * CHUNK + CHUNK + RING - 1, cy * CHUNK + CHUNK + RING - 1);
    let min_x = (x0 - y1) as f32 * TILE_W / 2.0 - PAD;
    let max_x = (x1 - y0) as f32 * TILE_W / 2.0 + TILE_W + PAD;
    let min_y = (x0 + y0) as f32 * TILE_H / 2.0 - PAD;
    let max_y = (x1 + y1) as f32 * TILE_H / 2.0 + TILE_H + PAD;
    (min_x, min_y, max_x - min_x, max_y - min_y)
}

struct Job {
    key: ChunkKey,
    hash: u64,
    map: TileMap,
    water: Option<Arc<Pixmap>>,
}

struct Done {
    key: ChunkKey,
    hash: u64,
    width: u32,
    height: u32,
    bgra: Vec<u8>,
}

#[derive(Default)]
struct Queue {
    jobs: Mutex<VecDeque<Job>>,
    cv: Condvar,
}

pub struct BakedChunk {
    pub image: Arc<RenderImage>,
    pub hash: u64,
    pub world: (f32, f32, f32, f32),
    pub last_used: u64,
    pub pixels: u64,
}

pub struct ChunkCache {
    queue: Arc<Queue>,
    results: Receiver<Done>,
    pending: HashSet<(ChunkKey, u64)>,
    pub chunks: HashMap<ChunkKey, BakedChunk>,
    frame: u64,
    water: Option<Arc<Pixmap>>,
    /// Images that should be released from the GPU atlas.
    pub garbage: Vec<Arc<RenderImage>>,
    hash_cache: HashMap<(i32, i32), u64>,
    hash_map_ptr: usize,
}

fn bake(job: &Job) -> Done {
    let (cx, cy) = (job.key.cx, job.key.cy);
    let scale = lod_scale(job.key.lod);
    let (wx, wy, ww, wh) = chunk_world_rect(cx, cy);
    let (pw, ph) = ((ww * scale).ceil() as u32, (wh * scale).ceil() as u32);
    let transform = Transform::from_scale(scale, scale).pre_translate(-wx, -wy);
    let mut canvas = Canvas::new(pw, ph, transform).expect("chunk pixmap");
    let detail_zoom = scale.max(0.25);
    let pc = PaintCtx {
        map: &job.map,
        zoom: detail_zoom,
        water: job.water.as_deref(),
    };
    let m = &job.map;
    let mut coords: Vec<(i32, i32)> = Vec::new();
    for y in (cy * CHUNK - RING)..(cy * CHUNK + CHUNK + RING) {
        for x in (cx * CHUNK - RING)..(cx * CHUNK + CHUNK + RING) {
            if x >= 0 && y >= 0 && x < m.size && y < m.size {
                coords.push((x, y));
            }
        }
    }
    coords.sort_by_key(|&(x, y)| (x + y, x));

    let adjacent_water = |x: i32, y: i32| {
        (-1..=1).any(|dy| (-1..=1).any(|dx| (dx != 0 || dy != 0) && m.is_water(x + dx, y + dy)))
    };
    let green_over_water = |x: i32, y: i32| {
        let t = m.get(x, y);
        matches!(t.kind, B::Grass | B::Empty | B::Tree) && adjacent_water(x, y)
    };
    let green_for_park = |x: i32, y: i32| {
        let t = m.get(x, y);
        matches!(t.kind, B::Park | B::ParkLarge) || (t.kind == B::Empty && t.park)
    };

    for &(x, y) in &coords {
        let skip_green = green_over_water(x, y) || green_for_park(x, y);
        tiles::draw_base_tile(&mut canvas, &pc, x, y, skip_green);
    }
    for &(x, y) in &coords {
        if m.get(x, y).kind == B::Water {
            tiles::draw_water(&mut canvas, &pc, x, y);
        }
    }
    if detail_zoom >= 0.4 {
        for &(x, y) in &coords {
            if m.get(x, y).kind == B::Water {
                tiles::draw_beach_on_water(&mut canvas, &pc, x, y);
            }
        }
    }
    for &(x, y) in &coords {
        if green_over_water(x, y) || green_for_park(x, y) {
            tiles::draw_green_base(&mut canvas, &pc, x, y);
        }
    }
    for &(x, y) in &coords {
        let t = m.get(x, y);
        if t.marina_pier {
            tiles::draw_water_under(&mut canvas, &pc, x, y);
        }
    }
    for &(x, y) in &coords {
        let t = m.get(x, y);
        if t.kind == B::Road {
            tiles::draw_road(&mut canvas, &pc, x, y);
            if t.rail_overlay {
                tiles::draw_rail_track(&mut canvas, &pc, x, y, true);
            }
        }
    }
    for &(x, y) in &coords {
        if m.get(x, y).kind == B::Bridge {
            tiles::draw_bridge(&mut canvas, &pc, x, y);
        }
    }
    for &(x, y) in &coords {
        if m.get(x, y).kind == B::Rail {
            tiles::draw_rail_tile(&mut canvas, &pc, x, y);
        }
    }
    for &(x, y) in &coords {
        let t = m.get(x, y);
        if t.grey_base && !t.marina_pier {
            tiles::draw_grey_base(&mut canvas, &pc, x, y);
        }
    }
    for &(x, y) in &coords {
        let t = m.get(x, y);
        if t.foundation {
            tiles::draw_foundation(&mut canvas, &pc, x, y);
        }
    }
    let (width, height, bgra) = canvas.into_bgra();
    Done {
        key: job.key,
        hash: job.hash,
        width,
        height,
        bgra,
    }
}

impl ChunkCache {
    pub fn new(water: Option<Arc<Pixmap>>) -> Self {
        let queue = Arc::new(Queue::default());
        let (tx, rx): (Sender<Done>, Receiver<Done>) = channel();
        let workers = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).clamp(1, 6))
            .unwrap_or(2);
        for i in 0..workers {
            let q = queue.clone();
            let tx = tx.clone();
            std::thread::Builder::new()
                .name(format!("chunk-baker-{i}"))
                .spawn(move || loop {
                    let job = {
                        let mut jobs = q.jobs.lock().unwrap();
                        loop {
                            if let Some(j) = jobs.pop_front() {
                                break j;
                            }
                            jobs = q.cv.wait(jobs).unwrap();
                        }
                    };
                    let done = bake(&job);
                    if tx.send(done).is_err() {
                        return;
                    }
                })
                .expect("spawn baker");
        }
        Self {
            queue,
            results: rx,
            pending: HashSet::new(),
            chunks: HashMap::new(),
            frame: 0,
            water,
            garbage: Vec::new(),
            hash_cache: HashMap::new(),
            hash_map_ptr: 0,
        }
    }

    /// Drops everything (new city loaded).
    pub fn clear(&mut self) {
        for (_, c) in self.chunks.drain() {
            self.garbage.push(c.image);
        }
        self.pending.clear();
        if let Ok(mut q) = self.queue.jobs.lock() {
            q.clear();
        }
        self.hash_cache.clear();
    }

    fn chunk_hash(map: &TileMap, cx: i32, cy: i32, lod: u8) -> u64 {
        // Depend on tiles up to 5 away (road merge analysis looks that far).
        let m = RING + 5;
        map.region_hash(cx * CHUNK - m, cy * CHUNK - m, cx * CHUNK + CHUNK + m, cy * CHUNK + CHUNK + m)
            ^ (lod as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ ((map.size as u64) << 48)
    }

    /// Collects finished bakes and schedules missing/outdated chunks.
    /// `visible` lists chunk coords in priority order.
    pub fn update(&mut self, map: &TileMap, visible: &[(i32, i32)], lod: u8) {
        self.frame += 1;
        while let Ok(done) = self.results.try_recv() {
            self.pending.remove(&(done.key, done.hash));
            let Some(buf) = ImageBuffer::from_raw(done.width, done.height, done.bgra) else {
                continue;
            };
            let image = Arc::new(RenderImage::new(SmallVec::from_elem(Frame::new(buf), 1)));
            let world = chunk_world_rect(done.key.cx, done.key.cy);
            let pixels = done.width as u64 * done.height as u64;
            if let Some(old) = self.chunks.insert(
                done.key,
                BakedChunk {
                    image,
                    hash: done.hash,
                    world,
                    last_used: self.frame,
                    pixels,
                },
            ) {
                self.garbage.push(old.image);
            }
        }

        let ptr = Arc::as_ptr(&map.tiles) as usize;
        if ptr != self.hash_map_ptr {
            self.hash_map_ptr = ptr;
            self.hash_cache.clear();
        }
        let mut jobs = Vec::new();
        for &(cx, cy) in visible {
            let key = ChunkKey { cx, cy, lod };
            let base = *self
                .hash_cache
                .entry((cx, cy))
                .or_insert_with(|| Self::chunk_hash(map, cx, cy, 0));
            let hash = base ^ (lod as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            if let Some(c) = self.chunks.get_mut(&key) {
                c.last_used = self.frame;
                if c.hash == hash {
                    continue;
                }
            }
            // Keep lower/higher LOD versions alive while waiting.
            for l in 0..=4u8 {
                if let Some(c) = self.chunks.get_mut(&ChunkKey { cx, cy, lod: l }) {
                    c.last_used = self.frame;
                }
            }
            if self.pending.insert((key, hash)) {
                jobs.push(Job {
                    key,
                    hash,
                    map: map.clone(),
                    water: self.water.clone(),
                });
            }
        }
        if !jobs.is_empty() {
            if let Ok(mut q) = self.queue.jobs.lock() {
                // Newest requests first; drop stale ones for chunks no longer wanted.
                let wanted: HashSet<ChunkKey> = jobs.iter().map(|j| j.key).collect();
                q.retain(|j| !wanted.contains(&j.key));
                for j in jobs.into_iter().rev() {
                    q.push_front(j);
                }
                q.truncate(256);
            }
            self.queue.cv.notify_all();
        }
        self.evict();
    }

    fn evict(&mut self) {
        let total: u64 = self.chunks.values().map(|c| c.pixels).sum();
        if total <= PIXEL_BUDGET {
            return;
        }
        let mut entries: Vec<(ChunkKey, u64, u64)> =
            self.chunks.iter().map(|(k, c)| (*k, c.last_used, c.pixels)).collect();
        entries.sort_by_key(|e| e.1);
        let mut over = total - PIXEL_BUDGET;
        for (k, used, px) in entries {
            if used >= self.frame || over == 0 {
                break;
            }
            if let Some(c) = self.chunks.remove(&k) {
                self.garbage.push(c.image);
            }
            over = over.saturating_sub(px);
        }
    }

    /// Best available baked image for a chunk, preferring `lod`.
    pub fn best(&self, cx: i32, cy: i32, lod: u8) -> Option<&BakedChunk> {
        if let Some(c) = self.chunks.get(&ChunkKey { cx, cy, lod }) {
            return Some(c);
        }
        let mut order: Vec<u8> = (0..=4).filter(|l| *l != lod).collect();
        order.sort_by_key(|l| (*l as i32 - lod as i32).abs() * 2 + (*l < lod) as i32);
        order
            .into_iter()
            .find_map(|l| self.chunks.get(&ChunkKey { cx, cy, lod: l }))
    }

}

/// Chunk coordinates intersecting a world-space rectangle.
pub fn visible_chunks(grid_size: i32, view: (f32, f32, f32, f32)) -> Vec<(i32, i32)> {
    let n = (grid_size + CHUNK - 1) / CHUNK;
    let (vx, vy, vw, vh) = view;
    let mut out = Vec::new();
    for cy in 0..n {
        for cx in 0..n {
            let (x, y, w, h) = chunk_world_rect(cx, cy);
            if x < vx + vw && x + w > vx && y < vy + vh && y + h > vy {
                out.push((cx, cy));
            }
        }
    }
    let (ccx, ccy) = (vx + vw / 2.0, vy + vh / 2.0);
    out.sort_by(|a, b| {
        let da = chunk_center_dist(*a, ccx, ccy);
        let db = chunk_center_dist(*b, ccx, ccy);
        da.total_cmp(&db)
    });
    out
}

fn chunk_center_dist((cx, cy): (i32, i32), x: f32, y: f32) -> f32 {
    let (wx, wy) = grid_to_world((cx * CHUNK + CHUNK / 2) as f32, (cy * CHUNK + CHUNK / 2) as f32);
    (wx - x).powi(2) + (wy - y).powi(2)
}
