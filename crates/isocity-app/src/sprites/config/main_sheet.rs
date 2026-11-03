//! Cells and offsets on the main building sheets, including construction
//! and abandoned variants.

use isocity_core::BuildingType as B;
use super::RC;

/// Main-sheet sprite order (row-major, 5 columns).
const SPRITE_ORDER: [&str; 30] = [
    "residential", "commercial", "industrial", "fire_station", "hospital",
    "park", "park_large", "tennis", "police_station", "school",
    "university", "water_tower", "power_plant", "stadium", "space_program",
    "tree", "house_medium", "mansion", "house_small", "shop_medium",
    "shop_small", "warehouse", "factory_small", "factory_medium", "factory_large",
    "airport", "subway_station", "city_hall", "museum", "amusement_park",
];

/// Sprite key on the main sheet for a building type.
pub fn sprite_key(b: B) -> Option<&'static str> {
    Some(match b {
        B::HouseSmall => "house_small",
        B::HouseMedium => "house_medium",
        B::Mansion => "mansion",
        B::ApartmentLow | B::ApartmentHigh => "residential",
        B::ShopSmall => "shop_small",
        B::ShopMedium => "shop_medium",
        B::OfficeLow | B::OfficeHigh | B::Mall => "commercial",
        B::FactorySmall => "factory_small",
        B::FactoryMedium => "factory_medium",
        B::FactoryLarge => "factory_large",
        B::Warehouse => "warehouse",
        B::PoliceStation => "police_station",
        B::FireStation => "fire_station",
        B::Hospital => "hospital",
        B::School => "school",
        B::University => "university",
        B::Park => "park",
        B::ParkLarge => "park_large",
        B::Tennis => "tennis",
        B::PowerPlant => "power_plant",
        B::WaterTower => "water_tower",
        B::Stadium => "stadium",
        B::Museum => "museum",
        B::Airport => "airport",
        B::SpaceProgram => "space_program",
        B::Tree => "tree",
        B::SubwayStation => "subway_station",
        B::CityHall => "city_hall",
        B::AmusementPark => "amusement_park",
        _ => return None,
    })
}

pub(super) fn main_cell(key: &str) -> Option<RC> {
    let i = SPRITE_ORDER.iter().position(|k| *k == key)? as u32;
    Some((i / 5, i % 5))
}

pub(super) fn vertical_offset(key: &str) -> f32 {
    match key {
        "residential" | "commercial" => -0.4,
        "industrial" => -0.5,
        "factory_small" => -0.25,
        "factory_medium" => -0.3,
        "factory_large" => -1.15,
        "water_tower" => -0.5,
        "house_medium" => -0.3,
        "mansion" => -0.35,
        "house_small" => -0.3,
        "shop_medium" => -0.15,
        "shop_small" => -0.3,
        "warehouse" => -0.4,
        "airport" => -1.5,
        "water" => -0.2,
        "subway_station" => -0.4,
        "fire_station" => -0.3,
        "police_station" => -0.2,
        "hospital" => -0.65,
        "school" => -0.35,
        "power_plant" => -0.3,
        "park" => -0.125,
        "park_large" => -0.77,
        "tennis" => -0.2,
        "city_hall" => -0.6,
        "amusement_park" => -1.5,
        "space_program" => -0.95,
        "university" => -0.55,
        "stadium" => -1.2,
        "museum" => -1.0,
        "tree" => -0.3,
        _ => 0.0,
    }
}

pub(super) fn horizontal_offset(key: &str) -> f32 {
    match key {
        "city_hall" => 0.1,
        _ => 0.0,
    }
}

pub(super) fn building_vertical_offset(b: B) -> Option<f32> {
    Some(match b {
        B::HouseSmall => -0.2,
        B::HouseMedium => -0.05,
        B::OfficeLow => -0.7,
        B::OfficeHigh => -0.7,
        B::Mall => -1.5,
        B::ApartmentLow => -0.9,
        B::ApartmentHigh => -0.60,
        _ => return None,
    })
}

pub(super) fn construction_vertical_offset(b: B) -> Option<f32> {
    Some(match b {
        B::WaterTower => -0.1,
        B::ApartmentHigh => -0.4,
        B::ApartmentLow => -0.5,
        B::Mall => -1.0,
        B::OfficeHigh => -0.5,
        B::OfficeLow => -0.4,
        B::Hospital => -0.7,
        B::Tennis => -0.2,
        _ => return None,
    })
}

pub(super) fn construction_scale(b: B) -> Option<f32> {
    Some(match b {
        B::Mall => 0.92,
        B::OfficeHigh => 0.80,
        B::ApartmentHigh => 0.65,
        B::ApartmentLow => 0.80,
        _ => return None,
    })
}

pub(super) fn abandoned_vertical_offset(b: B) -> Option<f32> {
    Some(match b {
        B::ApartmentLow => -0.45,
        B::ApartmentHigh => -0.35,
        B::HouseMedium => -0.05,
        B::HouseSmall => -0.05,
        B::Mansion => -0.25,
        B::OfficeHigh => -0.2,
        B::Tree => -0.3,
        B::FactorySmall => -0.05,
        _ => return None,
    })
}
