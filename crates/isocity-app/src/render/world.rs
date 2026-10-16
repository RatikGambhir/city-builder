//! Paints the map: ground chunks and depth-sorted building sprites.

use super::chunks::{self, ChunkCache};
use super::shapes;
use super::tiles::TileMap;
use crate::iso::{grid_to_world, Camera, TILE_H, TILE_W};
use crate::sprites::config::{self, SpritePack};
use crate::sprites::SpriteCache;
use gpui::{px, size, Bounds, Corners, Pixels, Point, Window};
use isocity_core::{place, BuildingType as B, GameState};
use std::sync::Arc;

/// A building sprite ready to paint, sorted by depth.
pub struct BuildingDraw {
    pub depth: i32,
    pub x: i32,
    pub y: i32,
}

pub struct WorldRenderer {
    pub sprites: SpriteCache,
    pub chunks: ChunkCache,
    pub tile_map: Option<TileMap>,
    map_version: u64,
    map_id: String,
    pub pack: SpritePack,
    pub scale_factor: f32,
}

fn load_water() -> Option<Arc<tiny_skia::Pixmap>> {
    let bytes = crate::assets::sprite_bytes("water.webp")?;
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    let mut pm = tiny_skia::Pixmap::new(w, h)?;
    for (dst, src) in pm.pixels_mut().iter_mut().zip(img.pixels()) {
        let [r, g, b, a] = src.0;
        *dst = tiny_skia::ColorU8::from_rgba(r, g, b, a).premultiply();
    }
    Some(Arc::new(pm))
}

/// Screen-space mapping for one frame.
#[derive(Clone, Copy)]
pub struct View {
    pub origin_x: f32,
    pub origin_y: f32,
    pub width: f32,
    pub height: f32,
    pub cam: Camera,
}

impl View {
    #[inline]
    pub fn to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        let (x, y) = self.cam.world_to_screen(wx, wy);
        (x + self.origin_x, y + self.origin_y)
    }

    /// Visible world rectangle (x, y, w, h).
    pub fn world_rect(&self) -> (f32, f32, f32, f32) {
        let (x0, y0) = self.cam.screen_to_world(0.0, 0.0);
        (x0, y0, self.width / self.cam.zoom, self.height / self.cam.zoom)
    }

    pub fn bounds(&self, x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
        Bounds::new(Point::new(px(x), px(y)), size(px(w), px(h)))
    }
}

impl WorldRenderer {
    pub fn new() -> Self {
        Self {
            sprites: SpriteCache::new(),
            chunks: ChunkCache::new(load_water()),
            tile_map: None,
            map_version: u64::MAX,
            map_id: String::new(),
            pack: SpritePack::Default,
            scale_factor: 1.0,
        }
    }

    /// Rebuilds derived tile data when the city changed.
    pub fn sync(&mut self, state: &GameState) {
        if state.id != self.map_id {
            self.map_id = state.id.clone();
            self.chunks.clear();
            self.map_version = u64::MAX;
        }
        if self.map_version != state.game_version || self.tile_map.as_ref().map(|m| m.size) != Some(state.grid.size()) {
            self.map_version = state.game_version;
            self.tile_map = Some(TileMap::build(state));
        }
    }

    pub fn release_garbage(&mut self, window: &mut Window) {
        for img in self.chunks.garbage.drain(..) {
            let _ = window.drop_image(img);
        }
    }

    pub fn paint_background(&self, window: &mut Window, v: &View) {
        let half = v.height / 2.0;
        window.paint_quad(gpui::fill(
            v.bounds(v.origin_x, v.origin_y, v.width, half + 1.0),
            gpui::linear_gradient(
                180.0,
                gpui::linear_color_stop(gpui::rgb(0x0f1419), 0.0),
                gpui::linear_color_stop(gpui::rgb(0x141c24), 1.0),
            ),
        ));
        window.paint_quad(gpui::fill(
            v.bounds(v.origin_x, v.origin_y + half, v.width, v.height - half),
            gpui::linear_gradient(
                180.0,
                gpui::linear_color_stop(gpui::rgb(0x141c24), 0.0),
                gpui::linear_color_stop(gpui::rgb(0x1a2a1f), 1.0),
            ),
        ));
    }

    pub fn paint_ground(&mut self, window: &mut Window, v: &View, grid_size: i32) {
        let Some(map) = self.tile_map.clone() else {
            return;
        };
        let lod = chunks::lod_for(v.cam.zoom, self.scale_factor);
        let visible = chunks::visible_chunks(grid_size, v.world_rect());
        self.chunks.update(&map, &visible, lod);
        let mut draw_list: Vec<(i32, i32)> = visible.clone();
        // Paint back-to-front so overlapping chunk padding layers correctly.
        draw_list.sort_by_key(|&(cx, cy)| (cx + cy, cx));
        for (cx, cy) in draw_list {
            if let Some(c) = self.chunks.best(cx, cy, lod) {
                let (wx, wy, ww, wh) = c.world;
                let (sx, sy) = v.to_screen(wx, wy);
                let b = v.bounds(sx, sy, ww * v.cam.zoom, wh * v.cam.zoom);
                let _ = window.paint_image(b, Corners::default(), c.image.clone(), 0, false);
            }
        }
    }

    /// Building and tree sprites in the visible area, sorted back-to-front.
    pub fn collect_buildings(&self, state: &GameState, v: &View) -> Vec<BuildingDraw> {
        let (vx, vy, vw, vh) = v.world_rect();
        let mut out = Vec::new();
        let n = state.grid.size();
        for t in state.grid.iter() {
            let k = t.building.kind;
            if matches!(k, B::Grass | B::Empty | B::Water | B::Road | B::Bridge | B::Rail) {
                continue;
            }
            if config::is_foundation_phase(&t.building) && k.needs_construction() {
                continue;
            }
            let fp = k.size();
            let (sx, sy) = grid_to_world(t.x as f32, t.y as f32);
            let margin_x = TILE_W * fp.width as f32 * 1.5;
            let margin_top = TILE_H * 8.0 * fp.width as f32;
            if sx + TILE_W + margin_x < vx || sx - margin_x > vx + vw || sy - margin_top > vy + vh || sy + TILE_H * ((fp.width + fp.height) as f32) < vy {
                continue;
            }
            let _ = n;
            out.push(BuildingDraw {
                depth: t.x + t.y + fp.width + fp.height - 2,
                x: t.x,
                y: t.y,
            });
        }
        out.sort_by_key(|d| (d.depth, d.x));
        out
    }

    /// Paints one building sprite (and fire if burning).
    pub fn paint_building(&mut self, window: &mut Window, v: &View, state: &GameState, gx: i32, gy: i32, time: f32) {
        let tile = state.grid.at(gx, gy);
        let b = &tile.building;
        let kind = b.kind;
        let (sx, sy) = grid_to_world(gx as f32, gy as f32);
        let (w, h) = (TILE_W, TILE_H);
        if let Some(src) = config::select_source(self.pack, b, gx, gy) {
            let crop = config::crop_for(kind, &src);
            let fp = kind.size();
            let multi = fp.is_multi();
            let waterfront = kind.requires_water_adjacency();
            let road_mirror = if waterfront {
                false
            } else {
                let (has_road, flip) = place::road_adjacency(&state.grid, gx, gy, fp.width, fp.height);
                if has_road && !kind.is_terrain() && !kind.is_park() {
                    flip
                } else {
                    (gx * 47 + gy * 83).rem_euclid(100) < 50
                }
            };
            let flipped = b.flipped != road_mirror;
            if let Some(sprite) = self.sprites.cell(src.sheet, crop, flipped) {
                let scale = config::scale_for(kind, &src, b);
                let (voff, hoff) = config::offsets_for(kind, &src, b);
                let dest_w = w * 1.2 * scale;
                let dest_h = dest_w * (sprite.full_h / sprite.full_w);
                let (mut px0, mut py0) = (sx, sy);
                if multi {
                    let fx = (fp.width - 1) as f32;
                    let fy = (fp.height - 1) as f32;
                    px0 += (fx - fy) * (w / 2.0);
                    py0 += (fx + fy) * (h / 2.0);
                }
                let draw_x = px0 + w / 2.0 - dest_w / 2.0 + hoff * w;
                let mut push = if multi {
                    (fp.width + fp.height - 2) as f32 * h * 0.25
                } else {
                    dest_h * 0.15
                };
                push += voff * h;
                let draw_y = py0 + h - dest_h + push;
                let (tx, ty, tw, th) = sprite.trimmed_rect(draw_x, draw_y, dest_w, dest_h);
                let (ssx, ssy) = v.to_screen(tx, ty);
                let z = v.cam.zoom;
                let bounds = v.bounds(ssx, ssy, tw * z, th * z);
                let _ = window.paint_image(bounds, Corners::default(), sprite.image.clone(), 0, false);
            }
        }
        if b.on_fire {
            let flicker = (time * 12.0 + (gx * 7 + gy * 13) as f32).sin() * 0.15 + 1.0;
            let (fx, fy) = v.to_screen(sx + w / 2.0, sy - 10.0);
            let z = v.cam.zoom * flicker;
            shapes::ellipse(window, fx, fy, 18.0 * z, 25.0 * z, shapes::rgba(0xff6400, 0.5));
            shapes::ellipse(window, fx, fy + 5.0 * z, 10.0 * z, 15.0 * z, shapes::rgba(0xffc800, 0.8));
            shapes::ellipse(window, fx, fy + 8.0 * z, 5.0 * z, 8.0 * z, shapes::rgba(0xffffc8, 0.9));
        }
    }
}
