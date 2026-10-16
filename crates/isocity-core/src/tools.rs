//! Player tools (what the sidebar offers) and their costs.

use crate::buildings::BuildingType;
use crate::types::ZoneType;
use serde::{Deserialize, Serialize};

macro_rules! tools {
    ($($variant:ident => ($name:expr, $cost:expr, $desc:expr)),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
        #[serde(rename_all = "snake_case")]
        pub enum Tool {
            #[default]
            $($variant,)*
        }

        impl Tool {
            pub const ALL: &'static [Tool] = &[$(Tool::$variant,)*];

            pub fn name(self) -> &'static str {
                match self { $(Tool::$variant => $name,)* }
            }

            pub fn cost(self) -> i64 {
                match self { $(Tool::$variant => $cost,)* }
            }

            pub fn description(self) -> &'static str {
                match self { $(Tool::$variant => $desc,)* }
            }
        }
    };
}

tools! {
    Select => ("Select", 0, "Click to view tile info"),
    Bulldoze => ("Bulldoze", 10, "Remove buildings and zones"),
    Road => ("Road", 25, "Connect your city"),
    Rail => ("Rail", 40, "Build railway tracks"),
    Subway => ("Subway", 50, "Underground transit"),
    ExpandCity => ("Expand City", 0, "Add 15 tiles to each edge"),
    ShrinkCity => ("Shrink City", 0, "Remove 15 tiles from each edge"),
    Tree => ("Tree", 15, "Plant trees to improve environment"),
    ZoneResidential => ("Residential", 50, "Zone for housing"),
    ZoneCommercial => ("Commercial", 50, "Zone for shops and offices"),
    ZoneIndustrial => ("Industrial", 50, "Zone for factories"),
    ZoneDezone => ("De-zone", 0, "Remove zoning"),
    ZoneWater => ("Water Terraform", 50000, "Terraform land into water"),
    ZoneLand => ("Land Terraform", 50000, "Terraform water into land"),
    PoliceStation => ("Police", 500, "Increase safety"),
    FireStation => ("Fire Station", 500, "Fight fires"),
    Hospital => ("Hospital", 1000, "Improve health (2x2)"),
    School => ("School", 400, "Basic education (2x2)"),
    University => ("University", 2000, "Higher education (3x3)"),
    Park => ("Small Park", 150, "Boost happiness and land value (1x1)"),
    ParkLarge => ("Large Park", 600, "Large park (3x3)"),
    Tennis => ("Tennis Court", 200, "Recreation facility"),
    PowerPlant => ("Power Plant", 3000, "Generate electricity (2x2)"),
    WaterTower => ("Water Tower", 1000, "Provide water"),
    SubwayStation => ("Subway Station", 750, "Access to subway network"),
    RailStation => ("Rail Station", 1000, "Passenger and freight station"),
    Stadium => ("Stadium", 5000, "Boosts commercial demand (3x3)"),
    Museum => ("Museum", 4000, "Boosts commercial & residential demand (3x3)"),
    Airport => ("Airport", 10000, "Boosts commercial & industrial demand (4x4)"),
    SpaceProgram => ("Space Program", 15000, "Boosts industrial & residential demand (3x3)"),
    CityHall => ("City Hall", 6000, "Boosts all demand types (2x2)"),
    AmusementPark => ("Amusement Park", 12000, "Major boost to commercial demand (4x4)"),
    BasketballCourts => ("Basketball Courts", 250, "Outdoor basketball facility"),
    PlaygroundSmall => ("Small Playground", 200, "Children's playground"),
    PlaygroundLarge => ("Large Playground", 350, "Large playground with more equipment (2x2)"),
    BaseballFieldSmall => ("Baseball Field", 800, "Local baseball diamond (2x2)"),
    SoccerFieldSmall => ("Soccer Field", 400, "Soccer/football pitch"),
    FootballField => ("Football Field", 1200, "Football stadium (2x2)"),
    BaseballStadium => ("Baseball Stadium", 6000, "Professional baseball venue (3x3)"),
    CommunityCenter => ("Community Center", 500, "Local community hub"),
    OfficeBuildingSmall => ("Small Office", 600, "Small office building"),
    SwimmingPool => ("Swimming Pool", 450, "Public swimming facility"),
    SkatePark => ("Skate Park", 300, "Skateboarding park"),
    MiniGolfCourse => ("Mini Golf", 700, "Miniature golf course (2x2)"),
    BleachersField => ("Bleachers Field", 350, "Sports field with seating"),
    GoKartTrack => ("Go-Kart Track", 1000, "Racing entertainment (2x2)"),
    Amphitheater => ("Amphitheater", 1500, "Outdoor performance venue (2x2)"),
    GreenhouseGarden => ("Greenhouse Garden", 800, "Botanical greenhouse (2x2)"),
    AnimalPensFarm => ("Animal Pens", 400, "Petting zoo / farm animals"),
    CabinHouse => ("Cabin House", 300, "Rustic cabin retreat"),
    Campground => ("Campground", 250, "Outdoor camping area"),
    MarinaDocksSmall => ("Marina", 1200, "Boat docks (2x2, must be placed next to water)"),
    PierLarge => ("Pier", 600, "Waterfront pier (must be placed next to water)"),
    RollerCoasterSmall => ("Roller Coaster", 3000, "Thrill ride (2x2)"),
    CommunityGarden => ("Community Garden", 200, "Shared gardening space"),
    PondPark => ("Pond Park", 350, "Park with scenic pond"),
    ParkGate => ("Park Gate", 150, "Decorative park entrance"),
    MountainLodge => ("Mountain Lodge", 1500, "Nature retreat lodge (2x2)"),
    MountainTrailhead => ("Trailhead", 400, "Hiking trail entrance (3x3)"),
}

impl Tool {
    /// The building this tool places, if any.
    pub fn building(self) -> Option<BuildingType> {
        use BuildingType as B;
        Some(match self {
            Tool::Road => B::Road,
            Tool::Rail => B::Rail,
            Tool::Tree => B::Tree,
            Tool::PoliceStation => B::PoliceStation,
            Tool::FireStation => B::FireStation,
            Tool::Hospital => B::Hospital,
            Tool::School => B::School,
            Tool::University => B::University,
            Tool::Park => B::Park,
            Tool::ParkLarge => B::ParkLarge,
            Tool::Tennis => B::Tennis,
            Tool::PowerPlant => B::PowerPlant,
            Tool::WaterTower => B::WaterTower,
            Tool::SubwayStation => B::SubwayStation,
            Tool::RailStation => B::RailStation,
            Tool::Stadium => B::Stadium,
            Tool::Museum => B::Museum,
            Tool::Airport => B::Airport,
            Tool::SpaceProgram => B::SpaceProgram,
            Tool::CityHall => B::CityHall,
            Tool::AmusementPark => B::AmusementPark,
            Tool::BasketballCourts => B::BasketballCourts,
            Tool::PlaygroundSmall => B::PlaygroundSmall,
            Tool::PlaygroundLarge => B::PlaygroundLarge,
            Tool::BaseballFieldSmall => B::BaseballFieldSmall,
            Tool::SoccerFieldSmall => B::SoccerFieldSmall,
            Tool::FootballField => B::FootballField,
            Tool::BaseballStadium => B::BaseballStadium,
            Tool::CommunityCenter => B::CommunityCenter,
            Tool::OfficeBuildingSmall => B::OfficeBuildingSmall,
            Tool::SwimmingPool => B::SwimmingPool,
            Tool::SkatePark => B::SkatePark,
            Tool::MiniGolfCourse => B::MiniGolfCourse,
            Tool::BleachersField => B::BleachersField,
            Tool::GoKartTrack => B::GoKartTrack,
            Tool::Amphitheater => B::Amphitheater,
            Tool::GreenhouseGarden => B::GreenhouseGarden,
            Tool::AnimalPensFarm => B::AnimalPensFarm,
            Tool::CabinHouse => B::CabinHouse,
            Tool::Campground => B::Campground,
            Tool::MarinaDocksSmall => B::MarinaDocksSmall,
            Tool::PierLarge => B::PierLarge,
            Tool::RollerCoasterSmall => B::RollerCoasterSmall,
            Tool::CommunityGarden => B::CommunityGarden,
            Tool::PondPark => B::PondPark,
            Tool::ParkGate => B::ParkGate,
            Tool::MountainLodge => B::MountainLodge,
            Tool::MountainTrailhead => B::MountainTrailhead,
            _ => return None,
        })
    }

    /// The zone this tool paints, if any. `Some(ZoneType::None)` de-zones.
    pub fn zone(self) -> Option<ZoneType> {
        match self {
            Tool::ZoneResidential => Some(ZoneType::Residential),
            Tool::ZoneCommercial => Some(ZoneType::Commercial),
            Tool::ZoneIndustrial => Some(ZoneType::Industrial),
            Tool::ZoneDezone => Some(ZoneType::None),
            _ => None,
        }
    }

    /// Footprint size shown in the placement preview.
    pub fn size(self) -> i32 {
        self.building().map(|b| b.size().width).unwrap_or(1)
    }

    /// Tools that paint rectangles when dragged.
    pub fn is_area_tool(self) -> bool {
        matches!(
            self,
            Tool::ZoneResidential
                | Tool::ZoneCommercial
                | Tool::ZoneIndustrial
                | Tool::ZoneDezone
                | Tool::ZoneWater
                | Tool::ZoneLand
                | Tool::Bulldoze
                | Tool::Tree
        )
    }

    /// Tools that draw straight (L-shaped) lines when dragged.
    pub fn is_line_tool(self) -> bool {
        matches!(self, Tool::Road | Tool::Rail | Tool::Subway)
    }
}
