//! Crop rectangle, scale and screen offsets for a chosen sprite.

use isocity_core::{Building, BuildingType as B};
use super::{CellCrop, SpriteSource, Variant, is_construction_phase};
use super::main_sheet::{abandoned_vertical_offset, building_vertical_offset, construction_scale, construction_vertical_offset, horizontal_offset, sprite_key, vertical_offset};
use super::parks::{parks_construction_vertical_offset, parks_scale, parks_vertical_offset};
use super::services::services_vertical_offset;

/// Source rectangle adjustments (bleed between rows on the AI-generated sheets).
pub fn crop_for(kind: B, s: &SpriteSource) -> CellCrop {
    let (row, col) = (s.row, s.col);
    let mut dy = 0.0;
    let mut h = 1.0;
    match s.variant {
        Variant::Parks | Variant::ParksConstruction => match kind {
            B::MarinaDocksSmall => {
                dy = 0.15;
                h = 0.85
            }
            B::PierLarge => {
                dy = 0.2;
                h = 0.8
            }
            B::Amphitheater | B::CabinHouse | B::GoKartTrack => dy = 0.1,
            B::MiniGolfCourse => {
                dy = 0.2;
                h = 0.8
            }
            B::GreenhouseGarden => {
                dy = 0.1;
                h = 0.9
            }
            B::BleachersField => h = 1.1,
            _ => {}
        },
        Variant::Dense => match kind {
            B::Mall => dy = 0.12,
            B::FactoryLarge => {
                dy = 0.05;
                h = 0.95
            }
            B::ApartmentHigh => h = 1.05,
            B::OfficeHigh => {
                dy = 0.1;
                h = 0.98
            }
            B::OfficeLow => dy = 0.1,
            _ => {}
        },
        Variant::Modern => match kind {
            B::Mall => {
                dy = 0.15;
                h = if row == 3 { 0.95 } else { 0.85 }
            }
            B::ApartmentHigh => h = 1.05,
            _ => {}
        },
        Variant::Station => match row {
            2 => dy = 0.1,
            3 => {
                dy = 0.1;
                h = 0.95
            }
            4 => {
                dy = 0.1;
                h = 0.9
            }
            _ => {}
        },
        Variant::Mansion => h = 1.20,
        Variant::Farm | Variant::Shop | Variant::Services | Variant::Infrastructure => {}
        Variant::Normal | Variant::Construction | Variant::Abandoned => {
            let key = sprite_key(kind).unwrap_or("");
            // Rows 1-4 of the main sheets include content bleeding from above.
            dy = match row {
                1 | 2 => 0.1 * row as f32,
                3 => 0.1,
                4 => 0.05,
                _ => 0.0,
            };
            if key == "residential" || key == "commercial" {
                h = 1.1;
            }
            if key == "space_program" {
                h = 0.92;
            }
            if kind == B::FactoryLarge {
                h -= 0.08;
            }
        }
    }
    CellCrop { row, col, dy, h }
}

pub fn scale_for(kind: B, s: &SpriteSource, b: &Building) -> f32 {
    let fp = kind.size();
    let mut scale = if fp.is_multi() { fp.width.max(fp.height) as f32 } else { 1.0 };
    scale *= match kind {
        B::Airport => 1.0,
        B::School => 1.05,
        B::University => 0.95,
        B::SpaceProgram => 1.06,
        B::Stadium => 0.7,
        B::WaterTower => 0.9,
        B::SubwayStation => 0.7,
        B::PoliceStation | B::FireStation => 0.97,
        B::Hospital => 0.9,
        B::HouseSmall => 1.08,
        B::ApartmentLow => 1.15,
        B::ApartmentHigh => 1.38,
        B::OfficeHigh => 1.20,
        _ => 1.0,
    };
    match s.variant {
        Variant::Dense => {
            if kind == B::Mall {
                scale *= 0.85;
            }
            scale *= match kind {
                B::ApartmentHigh => 0.90,
                B::OfficeHigh => 1.3,
                _ => 1.0,
            };
            if kind == B::OfficeHigh && s.row == 2 && (s.col == 1 || s.col == 2) {
                scale *= 0.7;
            }
        }
        Variant::Modern => {
            if kind == B::Mall {
                scale *= 0.85;
            }
            if kind == B::ApartmentHigh {
                scale *= 0.90;
            }
        }
        Variant::Shop => scale *= 0.90,
        Variant::Station => scale *= 0.85,
        Variant::Mansion => scale *= 0.90,
        Variant::Parks | Variant::ParksConstruction => scale *= parks_scale(kind),
        _ => {}
    }
    if is_construction_phase(b) {
        if let Some(c) = construction_scale(kind) {
            scale *= c;
        }
    }
    if b.abandoned && kind == B::FactoryLarge {
        scale *= 0.7;
    }
    scale * 0.8 // pack global scale
}

/// (vertical, horizontal) offsets in tile heights/widths.
pub fn offsets_for(kind: B, s: &SpriteSource, b: &Building) -> (f32, f32) {
    let construction = is_construction_phase(b);
    let parks = matches!(s.variant, Variant::Parks | Variant::ParksConstruction);
    let mut v = if construction && parks && parks_construction_vertical_offset(kind).is_some() {
        parks_construction_vertical_offset(kind).unwrap()
    } else if construction && construction_vertical_offset(kind).is_some() {
        construction_vertical_offset(kind).unwrap()
    } else if b.abandoned && abandoned_vertical_offset(kind).is_some() {
        abandoned_vertical_offset(kind).unwrap()
    } else if parks {
        parks_vertical_offset(kind)
    } else if s.variant == Variant::Dense
        && matches!(kind, B::ApartmentHigh | B::FactoryLarge | B::Mall | B::OfficeLow | B::OfficeHigh)
    {
        let mut v = match kind {
            B::ApartmentHigh => -0.80,
            B::FactoryLarge => -1.15,
            B::Mall => -1.5,
            B::OfficeLow => -0.4,
            _ => -0.7,
        };
        v += match (kind, s.row, s.col) {
            (B::Mall, 2, 4) => 0.5,
            (B::Mall, 3, 0) => 0.3,
            (B::Mall, 3, 1) => 0.5,
            (B::Mall, 3, 2) => 0.3,
            (B::OfficeHigh, 2, 0) => 0.2,
            (B::OfficeHigh, 2, 1) => 0.1,
            (B::OfficeHigh, 2, 2) => 0.1,
            _ => 0.0,
        };
        v
    } else if s.variant == Variant::Modern && matches!(kind, B::ApartmentHigh | B::Mall) {
        if kind == B::Mall {
            -1.3
        } else {
            -0.80
        }
    } else if s.variant == Variant::Farm {
        -0.25
    } else if s.variant == Variant::Shop {
        -0.35
    } else if s.variant == Variant::Station {
        -0.6
    } else if s.variant == Variant::Services {
        services_vertical_offset(kind)
    } else if s.variant == Variant::Infrastructure {
        -0.5
    } else if s.variant == Variant::Mansion {
        -0.65
    } else if let Some(o) = building_vertical_offset(kind) {
        o
    } else {
        sprite_key(kind).map(vertical_offset).unwrap_or(0.0)
    };
    if kind == B::Hospital {
        v -= 0.1;
    }
    let h = if parks || !matches!(s.variant, Variant::Normal | Variant::Construction | Variant::Abandoned | Variant::Dense | Variant::Modern) {
        0.0
    } else {
        sprite_key(kind).map(horizontal_offset).unwrap_or(0.0)
    };
    (v, h)
}
