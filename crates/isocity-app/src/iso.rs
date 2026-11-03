//! Isometric projection helpers. World space is "canvas pixels at zoom 1",
//! identical to the web version (64px wide tiles, 0.6 height ratio).

pub const TILE_W: f32 = 64.0;
pub const TILE_H: f32 = TILE_W * 0.6;
pub const ZOOM_MIN: f32 = 0.3;
pub const ZOOM_MAX: f32 = 5.0;

/// Top-left of the tile's bounding box.
#[inline]
pub fn grid_to_world(x: f32, y: f32) -> (f32, f32) {
    ((x - y) * (TILE_W / 2.0), (x + y) * (TILE_H / 2.0))
}

/// Center of the tile diamond.
#[inline]
pub fn tile_center(x: f32, y: f32) -> (f32, f32) {
    let (sx, sy) = grid_to_world(x, y);
    (sx + TILE_W / 2.0, sy + TILE_H / 2.0)
}

/// World position to the nearest tile (by diamond center).
pub fn world_to_grid(wx: f32, wy: f32) -> (i32, i32) {
    let ax = wx - TILE_W / 2.0;
    let ay = wy - TILE_H / 2.0;
    let gx = (ax / (TILE_W / 2.0) + ay / (TILE_H / 2.0)) / 2.0;
    let gy = (ay / (TILE_H / 2.0) - ax / (TILE_W / 2.0)) / 2.0;
    (gx.round() as i32, gy.round() as i32)
}

/// Fractional grid coordinates of a world point.
pub fn world_to_grid_f(wx: f32, wy: f32) -> (f32, f32) {
    let ax = wx - TILE_W / 2.0;
    let ay = wy - TILE_H / 2.0;
    (
        (ax / (TILE_W / 2.0) + ay / (TILE_H / 2.0)) / 2.0,
        (ay / (TILE_H / 2.0) - ax / (TILE_W / 2.0)) / 2.0,
    )
}

/// Camera: `screen = world * zoom + offset` (in logical pixels relative to
/// the map view's origin).
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub offset_x: f32,
    pub offset_y: f32,
    pub zoom: f32,
}

impl Camera {
    pub fn world_to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        (wx * self.zoom + self.offset_x, wy * self.zoom + self.offset_y)
    }

    pub fn screen_to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        ((sx - self.offset_x) / self.zoom, (sy - self.offset_y) / self.zoom)
    }

    /// Centers the camera on a tile for a view of `w x h`.
    pub fn center_on(&mut self, gx: f32, gy: f32, w: f32, h: f32) {
        let (cx, cy) = tile_center(gx, gy);
        self.offset_x = w / 2.0 - cx * self.zoom;
        self.offset_y = h / 2.0 - cy * self.zoom;
    }

    /// Zooms around a screen-space anchor point.
    pub fn zoom_at(&mut self, new_zoom: f32, ax: f32, ay: f32) {
        let z = new_zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        let (wx, wy) = self.screen_to_world(ax, ay);
        self.zoom = z;
        self.offset_x = ax - wx * z;
        self.offset_y = ay - wy * z;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for (x, y) in [(0, 0), (5, 9), (40, 3)] {
            let (cx, cy) = tile_center(x as f32, y as f32);
            assert_eq!(world_to_grid(cx, cy), (x, y));
        }
    }
}
