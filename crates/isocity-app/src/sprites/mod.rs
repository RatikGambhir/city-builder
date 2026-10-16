//! Sprite sheet decoding, background keying and cropping into GPU images.

pub mod config;

use crate::assets;
use config::{CellCrop, Sheet};
use gpui::{RenderImage, Window};
use image::{Frame, ImageBuffer, Rgba, RgbaImage};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// The sheets use a solid red background that is keyed out at load time,
/// exactly like the web version's `filterBackgroundColor`.
const KEY_THRESHOLD: f32 = 155.0;

fn decode_sheet(sheet: Sheet) -> Option<RgbaImage> {
    let bytes = assets::sprite_bytes(sheet.file())?;
    let mut img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    let dist = |p: &Rgba<u8>| {
        let [r, g, b, _] = p.0;
        ((r as f32 - 255.0).powi(2) + (g as f32).powi(2) + (b as f32).powi(2)).sqrt()
    };
    let mut keyed = vec![false; (w * h) as usize];
    for (i, p) in img.pixels_mut().enumerate() {
        if p.0[3] < 8 || dist(p) <= KEY_THRESHOLD {
            p.0 = [0, 0, 0, 0];
            keyed[i] = true;
        }
    }
    // Soften the anti-aliased rim: pixels touching the keyed background that
    // are still reddish get partial alpha with the red background un-mixed.
    const SOFT: f32 = 60.0;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if keyed[i] {
                continue;
            }
            let edge = (x > 0 && keyed[i - 1])
                || (x + 1 < w && keyed[i + 1])
                || (y > 0 && keyed[i - w as usize])
                || (y + 1 < h && keyed[i + w as usize]);
            if !edge {
                continue;
            }
            let p = img.get_pixel_mut(x, y);
            let d = dist(p);
            let [r0, g0, b0, _] = p.0;
            let reddish = r0 > 170 && (r0 as i32 - g0 as i32) > 70 && (r0 as i32 - b0 as i32) > 70;
            if d >= KEY_THRESHOLD + SOFT || !reddish {
                continue;
            }
            let a = ((d - KEY_THRESHOLD) / SOFT).clamp(0.05, 1.0);
            let [r, g, b, _] = p.0;
            let unmix = |c: u8, bg: f32| ((c as f32 - (1.0 - a) * bg) / a).clamp(0.0, 255.0) as u8;
            p.0 = [unmix(r, 255.0), unmix(g, 0.0), unmix(b, 0.0), (a * 255.0) as u8];
        }
    }
    Some(img)
}

/// Decoded sheets shared with a background loader thread.
#[derive(Default)]
pub struct SheetStore {
    sheets: Mutex<HashMap<Sheet, Arc<RgbaImage>>>,
    requested: Mutex<HashSet<Sheet>>,
}

impl SheetStore {
    pub fn get(&self, sheet: Sheet) -> Option<Arc<RgbaImage>> {
        self.sheets.lock().ok()?.get(&sheet).cloned()
    }

    /// Starts decoding `sheets` on a background thread (no-op if requested).
    pub fn request(self: &Arc<Self>, sheets: &[Sheet]) {
        let mut todo = Vec::new();
        if let Ok(mut req) = self.requested.lock() {
            for s in sheets {
                if req.insert(*s) {
                    todo.push(*s);
                }
            }
        }
        if todo.is_empty() {
            return;
        }
        let store = self.clone();
        std::thread::spawn(move || {
            for s in todo {
                if let Some(img) = decode_sheet(s) {
                    if let Ok(mut map) = store.sheets.lock() {
                        map.insert(s, Arc::new(img));
                    }
                }
            }
        });
    }
}

/// A cropped sprite. `trim_*` describe the opaque sub-rectangle actually
/// stored in `image`, relative to the full crop of `full_w x full_h`.
pub struct Sprite {
    pub image: Arc<RenderImage>,
    pub full_w: f32,
    pub full_h: f32,
    pub trim_x: f32,
    pub trim_y: f32,
    pub trim_w: f32,
    pub trim_h: f32,
}

impl Sprite {
    /// Destination bounds for drawing the full crop at `(x, y, w, h)`.
    pub fn trimmed_rect(&self, x: f32, y: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
        let sx = w / self.full_w;
        let sy = h / self.full_h;
        (x + self.trim_x * sx, y + self.trim_y * sy, self.trim_w * sx, self.trim_h * sy)
    }
}

/// Converts straight-alpha RGBA into a GPUI image (BGRA).
pub fn render_image_from_rgba(mut img: RgbaImage) -> Arc<RenderImage> {
    for p in img.pixels_mut() {
        p.0.swap(0, 2);
    }
    Arc::new(RenderImage::new(SmallVec::from_elem(Frame::new(img), 1)))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CropKey {
    sheet: Sheet,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    flipped: bool,
}

pub struct SpriteCache {
    pub store: Arc<SheetStore>,
    crops: HashMap<CropKey, Option<Arc<Sprite>>>,
}

impl SpriteCache {
    pub fn new() -> Self {
        let store = Arc::new(SheetStore::default());
        store.request(&[
            Sheet::Main,
            Sheet::Parks,
            Sheet::Services,
            Sheet::Shops,
            Sheet::Farms,
            Sheet::Dense,
            Sheet::Modern,
            Sheet::Construction,
            Sheet::ParksConstruction,
            Sheet::Abandoned,
            Sheet::Stations,
            Sheet::Infrastructure,
            Sheet::Mansions,
            Sheet::Planes,
        ]);
        Self {
            store,
            crops: HashMap::new(),
        }
    }

    /// Sprite for a sheet cell with the per-building crop adjustments.
    pub fn cell(&mut self, sheet: Sheet, crop: CellCrop, flipped: bool) -> Option<Arc<Sprite>> {
        let Some(img) = self.store.get(sheet) else {
            self.store.request(&[sheet]);
            return None;
        };
        let (cols, rows) = sheet.layout();
        let (iw, ih) = img.dimensions();
        let tw = iw / cols;
        let th = ih / rows;
        let x = crop.col * tw;
        let y = (crop.row as f32 * th as f32 + crop.dy * th as f32) as u32;
        let h = (crop.h * th as f32) as u32;
        self.rect(sheet, x, y, tw, h, flipped)
    }

    /// Sprite for an arbitrary pixel rectangle of a sheet.
    pub fn rect(&mut self, sheet: Sheet, x: u32, y: u32, w: u32, h: u32, flipped: bool) -> Option<Arc<Sprite>> {
        let key = CropKey { sheet, x, y, w, h, flipped };
        if let Some(s) = self.crops.get(&key) {
            return s.clone();
        }
        let img = self.store.get(sheet)?;
        let sprite = crop_sprite(&img, x, y, w, h, flipped).map(Arc::new);
        self.crops.insert(key, sprite.clone());
        sprite
    }

    /// Frees GPU memory for every cached crop (e.g. when switching themes).
    pub fn clear(&mut self, window: &mut Window, cx: &mut gpui::App) {
        for s in self.crops.drain().filter_map(|(_, s)| s) {
            let _ = window.drop_image(s.image.clone());
        }
        let _ = cx;
    }
}

fn crop_sprite(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32, flipped: bool) -> Option<Sprite> {
    let (iw, ih) = img.dimensions();
    if x >= iw || y >= ih {
        return None;
    }
    let w_c = w.min(iw - x);
    let h_c = h.min(ih - y);
    // Find the opaque bounds to keep textures small.
    let (mut minx, mut miny, mut maxx, mut maxy) = (u32::MAX, u32::MAX, 0, 0);
    for yy in 0..h_c {
        for xx in 0..w_c {
            if img.get_pixel(x + xx, y + yy).0[3] > 8 {
                minx = minx.min(xx);
                maxx = maxx.max(xx);
                miny = miny.min(yy);
                maxy = maxy.max(yy);
            }
        }
    }
    if minx == u32::MAX {
        return None;
    }
    let tw = maxx - minx + 1;
    let th = maxy - miny + 1;
    let mut out: RgbaImage = ImageBuffer::new(tw, th);
    for yy in 0..th {
        for xx in 0..tw {
            let sx = if flipped { maxx - xx } else { minx + xx };
            let p = *img.get_pixel(x + sx, y + miny + yy);
            out.put_pixel(xx, yy, Rgba(p.0));
        }
    }
    // Mirroring flips the trim rectangle around the crop's vertical axis.
    let trim_x = if flipped { w as f32 - (maxx + 1) as f32 } else { minx as f32 };
    Some(Sprite {
        image: render_image_from_rgba(out),
        full_w: w as f32,
        full_h: h as f32,
        trim_x,
        trim_y: miny as f32,
        trim_w: tw as f32,
        trim_h: th as f32,
    })
}
