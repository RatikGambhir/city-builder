//! Cells, offsets and scales on the parks sheet.

use isocity_core::BuildingType as B;
use super::RC;

pub fn parks_cell(b: B) -> Option<RC> {
    Some(match b {
        B::BasketballCourts => (0, 1),
        B::PlaygroundSmall => (0, 2),
        B::PlaygroundLarge => (0, 3),
        B::BaseballFieldSmall => (0, 4),
        B::SoccerFieldSmall => (1, 0),
        B::FootballField => (1, 1),
        B::BaseballStadium => (1, 2),
        B::CommunityCenter => (1, 3),
        B::OfficeBuildingSmall => (1, 4),
        B::SwimmingPool => (2, 0),
        B::SkatePark => (2, 1),
        B::MiniGolfCourse => (2, 2),
        B::BleachersField => (2, 3),
        B::GoKartTrack => (2, 4),
        B::Amphitheater => (3, 0),
        B::GreenhouseGarden => (3, 1),
        B::AnimalPensFarm => (3, 2),
        B::CabinHouse => (3, 3),
        B::Campground => (3, 4),
        B::MarinaDocksSmall => (4, 0),
        B::PierLarge => (4, 1),
        B::RollerCoasterSmall => (4, 4),
        B::CommunityGarden => (5, 0),
        B::PondPark => (5, 1),
        B::ParkGate => (5, 2),
        B::MountainLodge => (5, 3),
        B::MountainTrailhead => (5, 4),
        _ => return None,
    })
}

pub(super) fn parks_vertical_offset(b: B) -> f32 {
    match b {
        B::BasketballCourts => -0.25,
        B::PlaygroundSmall => -0.25,
        B::PlaygroundLarge => -0.60,
        B::BaseballFieldSmall => -0.55,
        B::SoccerFieldSmall => -0.20,
        B::FootballField => -0.55,
        B::BaseballStadium => -1.35,
        B::CommunityCenter => -0.2,
        B::OfficeBuildingSmall => -0.3,
        B::SwimmingPool => -0.20,
        B::SkatePark => -0.25,
        B::MiniGolfCourse => -0.60,
        B::BleachersField => -0.2,
        B::GoKartTrack => -0.30,
        B::Amphitheater => -0.40,
        B::GreenhouseGarden => -0.75,
        B::AnimalPensFarm => -0.25,
        B::CabinHouse => -0.2,
        B::Campground => -0.15,
        B::MarinaDocksSmall => -0.45,
        B::PierLarge => -0.1,
        B::RollerCoasterSmall => -0.50,
        B::CommunityGarden => -0.15,
        B::PondPark => -0.15,
        B::ParkGate => -0.15,
        B::MountainLodge => -0.55,
        B::MountainTrailhead => -1.0,
        _ => 0.0,
    }
}

pub(super) fn parks_scale(b: B) -> f32 {
    match b {
        B::BaseballStadium => 0.81,
        B::BaseballFieldSmall => 0.855,
        B::BasketballCourts => 0.9,
        B::FootballField => 0.855,
        B::SwimmingPool => 0.90,
        B::SoccerFieldSmall => 0.95,
        B::GoKartTrack => 0.92,
        B::MiniGolfCourse => 0.95,
        B::Amphitheater => 0.90,
        B::GreenhouseGarden => 0.90,
        _ => 1.0,
    }
}

pub(super) fn parks_construction_vertical_offset(b: B) -> Option<f32> {
    match b {
        B::BaseballFieldSmall | B::MountainLodge => Some(-0.55),
        _ => None,
    }
}
