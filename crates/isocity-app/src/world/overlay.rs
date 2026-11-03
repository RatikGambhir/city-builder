//! Map overlays (power, water, service coverage) and their painting.

use crate::iso::{grid_to_world, tile_center, TILE_H, TILE_W};
use crate::render::shapes::{self, rgba};
use crate::render::world::View;
use gpui::{Hsla, Window};
use isocity_core::{sim::effective_service_range, BuildingType as B, Session, Tool};
use super::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OverlayMode {
    None,
    Power,
    Water,
    Fire,
    Police,
    Health,
    Education,
    Subway,
}

impl OverlayMode {
    pub const ALL: [OverlayMode; 8] = [
        OverlayMode::None,
        OverlayMode::Power,
        OverlayMode::Water,
        OverlayMode::Fire,
        OverlayMode::Police,
        OverlayMode::Health,
        OverlayMode::Education,
        OverlayMode::Subway,
    ];

    pub fn icon(self) -> &'static str {
        match self {
            OverlayMode::None => "close",
            OverlayMode::Power => "power",
            OverlayMode::Water => "water",
            OverlayMode::Fire => "fire",
            OverlayMode::Police => "safety",
            OverlayMode::Health => "health",
            OverlayMode::Education => "education",
            OverlayMode::Subway => "subway",
        }
    }

    pub fn color(self) -> u32 {
        match self {
            OverlayMode::None => 0x60a5fa,
            OverlayMode::Power => 0xf59e0b,
            OverlayMode::Water => 0x3b82f6,
            OverlayMode::Fire => 0xef4444,
            OverlayMode::Police => 0x2563eb,
            OverlayMode::Health => 0x22c55e,
            OverlayMode::Education => 0xa855f7,
            OverlayMode::Subway => 0xeab308,
        }
    }

    fn building_types(self) -> &'static [B] {
        match self {
            OverlayMode::Power => &[B::PowerPlant],
            OverlayMode::Water => &[B::WaterTower],
            OverlayMode::Fire => &[B::FireStation],
            OverlayMode::Police => &[B::PoliceStation],
            OverlayMode::Health => &[B::Hospital],
            OverlayMode::Education => &[B::School, B::University],
            _ => &[],
        }
    }

    /// Overlay automatically shown while a related tool is selected.
    pub fn for_tool(tool: Tool) -> Option<OverlayMode> {
        Some(match tool {
            Tool::PowerPlant => OverlayMode::Power,
            Tool::WaterTower => OverlayMode::Water,
            Tool::FireStation => OverlayMode::Fire,
            Tool::PoliceStation => OverlayMode::Police,
            Tool::Hospital => OverlayMode::Health,
            Tool::School | Tool::University => OverlayMode::Education,
            Tool::SubwayStation | Tool::Subway => OverlayMode::Subway,
            _ => return None,
        })
    }
}

impl World {
    pub(super) fn paint_overlay(&self, window: &mut Window, v: &View) {
        let mode = if self.overlay == OverlayMode::None {
            OverlayMode::for_tool(self.tool).unwrap_or(OverlayMode::None)
        } else {
            self.overlay
        };
        if mode == OverlayMode::None {
            return;
        }
        let state = self.session.state();
        let s = &state.services;
        let (vx, vy, vw, vh) = v.world_rect();
        for t in state.grid.iter() {
            let (wx, wy) = grid_to_world(t.x as f32, t.y as f32);
            if wx + TILE_W < vx || wx > vx + vw || wy + TILE_H < vy || wy > vy + vh {
                continue;
            }
            let k = t.building.kind;
            let needs = !matches!(k, B::Empty | B::Grass | B::Water | B::Road | B::Rail | B::Tree);
            if s.police.is_empty() {
                continue;
            }
            let i = s.idx(t.x, t.y);
            let fill: Option<Hsla> = match mode {
                OverlayMode::Subway => {
                    if k == B::Water {
                        None
                    } else if t.has_subway {
                        Some(rgba(0xf59e0b, 0.7))
                    } else {
                        Some(rgba(0x281e14, 0.4))
                    }
                }
                _ if !needs || k == B::Bridge => None,
                OverlayMode::Power => (!t.building.powered).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Water => (!t.building.watered).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Fire => (s.fire[i] <= 0.0).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Police => (s.police[i] <= 0.0).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Health => (s.health[i] <= 0.0).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::Education => (s.education[i] <= 0.0).then(|| rgba(0xef4444, 0.45)),
                OverlayMode::None => None,
            };
            if let Some(c) = fill {
                shapes::fill_poly(window, &Self::diamond_screen(v, t.x, t.y), c);
            }
        }
        let color = mode.color();
        for t in state.grid.iter() {
            if !mode.building_types().contains(&t.building.kind) || t.building.under_construction() || t.building.abandoned {
                continue;
            }
            let Some(range) = effective_service_range(t.building.kind, t.building.level) else {
                continue;
            };
            let (cx, cy) = tile_center(t.x as f32, t.y as f32);
            let (sx, sy) = v.to_screen(cx, cy);
            let z = v.cam.zoom;
            let rx = range as f32 * TILE_W / 2.0 * z;
            let ry = range as f32 * TILE_H / 2.0 * z;
            let pts: Vec<(f32, f32)> = (0..64)
                .map(|i| {
                    let a = i as f32 / 64.0 * std::f32::consts::TAU;
                    (sx + rx * a.cos(), sy + ry * a.sin())
                })
                .collect();
            shapes::fill_poly(window, &pts, rgba(color, 0.12));
            shapes::stroke_poly(window, &pts, true, 2.0, rgba(color, 0.8));
            shapes::stroke_poly(window, &Self::diamond_screen(v, t.x, t.y), true, 3.0, rgba(color, 1.0));
            shapes::circle(window, sx, sy, 4.0, rgba(color, 1.0));
        }
    }
}
