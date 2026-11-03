//! Rows and offsets on the services sheet.

use isocity_core::BuildingType as B;

pub(super) fn services_row(b: B) -> Option<u32> {
    Some(match b {
        B::PoliceStation => 0,
        B::FireStation => 1,
        B::Hospital => 2,
        B::School => 3,
        B::University => 4,
        B::PowerPlant => 5,
        _ => return None,
    })
}

pub(super) fn services_vertical_offset(b: B) -> f32 {
    match b {
        B::PoliceStation | B::FireStation | B::School | B::PowerPlant => -0.3,
        B::Hospital => -0.50,
        B::University => -0.85,
        _ => 0.0,
    }
}
