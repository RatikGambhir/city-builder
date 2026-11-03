//! Building catalogue: types, stats, footprints and categories.

use serde::{Deserialize, Serialize};

macro_rules! building_types {
    ($($variant:ident),* $(,)?) => {
        /// Everything that can occupy a tile. Serialized in `snake_case`, which
        /// matches the IsoCity web save format.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default)]
        #[serde(rename_all = "snake_case")]
        pub enum BuildingType {
            #[default]
            $($variant,)*
        }

        impl BuildingType {
            pub const ALL: &'static [BuildingType] = &[$(BuildingType::$variant,)*];
        }
    };
}

building_types!(
    Grass,
    Empty,
    Water,
    Road,
    Bridge,
    Rail,
    Tree,
    // Residential
    HouseSmall,
    HouseMedium,
    Mansion,
    ApartmentLow,
    ApartmentHigh,
    // Commercial
    ShopSmall,
    ShopMedium,
    OfficeLow,
    OfficeHigh,
    Mall,
    // Industrial
    FactorySmall,
    FactoryMedium,
    FactoryLarge,
    Warehouse,
    // Services
    PoliceStation,
    FireStation,
    Hospital,
    School,
    University,
    Park,
    ParkLarge,
    Tennis,
    // Utilities
    PowerPlant,
    WaterTower,
    // Transportation
    SubwayStation,
    RailStation,
    // Special
    Stadium,
    Museum,
    Airport,
    SpaceProgram,
    CityHall,
    AmusementPark,
    // Parks sheet
    BasketballCourts,
    PlaygroundSmall,
    PlaygroundLarge,
    BaseballFieldSmall,
    SoccerFieldSmall,
    FootballField,
    BaseballStadium,
    CommunityCenter,
    OfficeBuildingSmall,
    SwimmingPool,
    SkatePark,
    MiniGolfCourse,
    BleachersField,
    GoKartTrack,
    Amphitheater,
    GreenhouseGarden,
    AnimalPensFarm,
    CabinHouse,
    Campground,
    MarinaDocksSmall,
    PierLarge,
    RollerCoasterSmall,
    CommunityGarden,
    PondPark,
    ParkGate,
    MountainLodge,
    MountainTrailhead,
);

/// Static economic stats per building type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BuildingStats {
    pub max_pop: u32,
    pub max_jobs: u32,
    pub pollution: f32,
    pub land_value: f32,
}

const fn st(max_pop: u32, max_jobs: u32, pollution: f32, land_value: f32) -> BuildingStats {
    BuildingStats {
        max_pop,
        max_jobs,
        pollution,
        land_value,
    }
}

/// Footprint in tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footprint {
    pub width: i32,
    pub height: i32,
}

impl Footprint {
    pub fn is_multi(self) -> bool {
        self.width > 1 || self.height > 1
    }
}

pub const RESIDENTIAL_BUILDINGS: [BuildingType; 5] = [
    BuildingType::HouseSmall,
    BuildingType::HouseMedium,
    BuildingType::Mansion,
    BuildingType::ApartmentLow,
    BuildingType::ApartmentHigh,
];
pub const COMMERCIAL_BUILDINGS: [BuildingType; 5] = [
    BuildingType::ShopSmall,
    BuildingType::ShopMedium,
    BuildingType::OfficeLow,
    BuildingType::OfficeHigh,
    BuildingType::Mall,
];
pub const INDUSTRIAL_BUILDINGS: [BuildingType; 5] = [
    BuildingType::FactorySmall,
    BuildingType::FactoryMedium,
    BuildingType::Warehouse,
    BuildingType::FactoryLarge,
    BuildingType::FactoryLarge,
];

impl BuildingType {
    pub fn name(self) -> &'static str {
        use BuildingType::*;
        match self {
            Grass => "Grass",
            Empty => "Empty",
            Water => "Water",
            Road => "Road",
            Bridge => "Bridge",
            Rail => "Rail",
            Tree => "Tree",
            HouseSmall => "Small House",
            HouseMedium => "House",
            Mansion => "Mansion",
            ApartmentLow => "Apartments",
            ApartmentHigh => "High-rise Apartments",
            ShopSmall => "Small Shop",
            ShopMedium => "Shop",
            OfficeLow => "Office",
            OfficeHigh => "Office Tower",
            Mall => "Mall",
            FactorySmall => "Small Factory",
            FactoryMedium => "Factory",
            FactoryLarge => "Large Factory",
            Warehouse => "Warehouse",
            PoliceStation => "Police Station",
            FireStation => "Fire Station",
            Hospital => "Hospital",
            School => "School",
            University => "University",
            Park => "Small Park",
            ParkLarge => "Large Park",
            Tennis => "Tennis Court",
            PowerPlant => "Power Plant",
            WaterTower => "Water Tower",
            SubwayStation => "Subway Station",
            RailStation => "Rail Station",
            Stadium => "Stadium",
            Museum => "Museum",
            Airport => "Airport",
            SpaceProgram => "Space Program",
            CityHall => "City Hall",
            AmusementPark => "Amusement Park",
            BasketballCourts => "Basketball Courts",
            PlaygroundSmall => "Small Playground",
            PlaygroundLarge => "Large Playground",
            BaseballFieldSmall => "Baseball Field",
            SoccerFieldSmall => "Soccer Field",
            FootballField => "Football Field",
            BaseballStadium => "Baseball Stadium",
            CommunityCenter => "Community Center",
            OfficeBuildingSmall => "Small Office",
            SwimmingPool => "Swimming Pool",
            SkatePark => "Skate Park",
            MiniGolfCourse => "Mini Golf",
            BleachersField => "Bleachers Field",
            GoKartTrack => "Go-Kart Track",
            Amphitheater => "Amphitheater",
            GreenhouseGarden => "Greenhouse Garden",
            AnimalPensFarm => "Animal Pens",
            CabinHouse => "Cabin House",
            Campground => "Campground",
            MarinaDocksSmall => "Marina",
            PierLarge => "Pier",
            RollerCoasterSmall => "Roller Coaster",
            CommunityGarden => "Community Garden",
            PondPark => "Pond Park",
            ParkGate => "Park Gate",
            MountainLodge => "Mountain Lodge",
            MountainTrailhead => "Trailhead",
        }
    }

    pub fn stats(self) -> BuildingStats {
        use BuildingType::*;
        match self {
            Empty | Grass => st(0, 0, 0.0, 0.0),
            Water => st(0, 0, 0.0, 5.0),
            Road => st(0, 0, 2.0, 0.0),
            Bridge => st(0, 0, 1.0, 5.0),
            Rail => st(0, 0, 1.0, -2.0),
            Tree => st(0, 0, -5.0, 2.0),
            HouseSmall => st(6, 0, 0.0, 10.0),
            HouseMedium => st(14, 0, 0.0, 22.0),
            Mansion => st(18, 0, 0.0, 60.0),
            ApartmentLow => st(120, 0, 2.0, 40.0),
            ApartmentHigh => st(260, 0, 3.0, 55.0),
            ShopSmall => st(0, 10, 1.0, 16.0),
            ShopMedium => st(0, 28, 2.0, 26.0),
            OfficeLow => st(0, 90, 2.0, 40.0),
            OfficeHigh => st(0, 210, 3.0, 55.0),
            Mall => st(0, 260, 6.0, 70.0),
            FactorySmall => st(0, 40, 15.0, -5.0),
            FactoryMedium => st(0, 90, 28.0, -10.0),
            FactoryLarge => st(0, 180, 55.0, -18.0),
            Warehouse => st(0, 60, 18.0, -6.0),
            PoliceStation => st(0, 20, 0.0, 15.0),
            FireStation => st(0, 20, 0.0, 10.0),
            Hospital => st(0, 80, 0.0, 25.0),
            School => st(0, 25, 0.0, 15.0),
            University => st(0, 100, 0.0, 35.0),
            Park => st(0, 2, -10.0, 20.0),
            ParkLarge => st(0, 6, -25.0, 50.0),
            Tennis => st(0, 1, -5.0, 15.0),
            PowerPlant => st(0, 30, 30.0, -20.0),
            WaterTower => st(0, 5, 0.0, 5.0),
            Stadium => st(0, 50, 5.0, 40.0),
            Museum => st(0, 40, 0.0, 45.0),
            Airport => st(0, 200, 20.0, 50.0),
            SpaceProgram => st(0, 150, 5.0, 80.0),
            SubwayStation => st(0, 15, 0.0, 25.0),
            RailStation => st(0, 25, 2.0, 20.0),
            CityHall => st(0, 60, 0.0, 50.0),
            AmusementPark => st(0, 100, 8.0, 60.0),
            BasketballCourts => st(0, 2, -3.0, 12.0),
            PlaygroundSmall => st(0, 1, -5.0, 15.0),
            PlaygroundLarge => st(0, 2, -8.0, 18.0),
            BaseballFieldSmall => st(0, 4, -10.0, 25.0),
            SoccerFieldSmall => st(0, 2, -5.0, 15.0),
            FootballField => st(0, 8, -8.0, 30.0),
            BaseballStadium => st(0, 60, 5.0, 45.0),
            CommunityCenter => st(0, 10, 0.0, 20.0),
            OfficeBuildingSmall => st(0, 25, 1.0, 22.0),
            SwimmingPool => st(0, 5, -5.0, 18.0),
            SkatePark => st(0, 2, -3.0, 12.0),
            MiniGolfCourse => st(0, 6, -8.0, 22.0),
            BleachersField => st(0, 3, -5.0, 15.0),
            GoKartTrack => st(0, 10, 5.0, 20.0),
            Amphitheater => st(0, 15, -5.0, 35.0),
            GreenhouseGarden => st(0, 8, -15.0, 28.0),
            AnimalPensFarm => st(0, 4, 2.0, 10.0),
            CabinHouse => st(4, 0, -3.0, 15.0),
            Campground => st(0, 3, -8.0, 12.0),
            MarinaDocksSmall => st(0, 8, 2.0, 25.0),
            PierLarge => st(0, 12, 1.0, 30.0),
            RollerCoasterSmall => st(0, 20, 3.0, 40.0),
            CommunityGarden => st(0, 2, -12.0, 18.0),
            PondPark => st(0, 2, -15.0, 22.0),
            ParkGate => st(0, 1, -2.0, 8.0),
            MountainLodge => st(0, 15, -5.0, 35.0),
            MountainTrailhead => st(0, 2, -10.0, 15.0),
        }
    }

    pub fn size(self) -> Footprint {
        use BuildingType::*;
        let (w, h) = match self {
            PowerPlant | Hospital | School | Mansion | ApartmentLow | ApartmentHigh
            | OfficeLow | OfficeHigh | FactoryMedium | Warehouse | CityHall
            | PlaygroundLarge | BaseballFieldSmall | FootballField | MiniGolfCourse
            | GoKartTrack | Amphitheater | GreenhouseGarden | MarinaDocksSmall
            | RollerCoasterSmall | MountainLodge | RailStation => (2, 2),
            Stadium | Museum | University | SpaceProgram | ParkLarge | Mall | FactoryLarge
            | BaseballStadium | MountainTrailhead => (3, 3),
            Airport | AmusementPark => (4, 4),
            _ => (1, 1),
        };
        Footprint {
            width: w,
            height: h,
        }
    }

    /// Terrain / infrastructure that is not a "building" for growth purposes.
    pub fn is_terrain(self) -> bool {
        use BuildingType::*;
        matches!(self, Grass | Empty | Water | Road | Bridge | Rail | Tree)
    }

    /// Types that complete instantly when placed.
    pub fn needs_construction(self) -> bool {
        use BuildingType::*;
        !matches!(self, Grass | Empty | Water | Road | Bridge | Tree)
    }

    pub fn requires_water_adjacency(self) -> bool {
        matches!(self, BuildingType::MarinaDocksSmall | BuildingType::PierLarge)
    }

    /// Recreation buildings that sit on green grass rather than concrete.
    pub fn is_park(self) -> bool {
        use BuildingType::*;
        matches!(
            self,
            Park | ParkLarge
                | Tennis
                | BasketballCourts
                | PlaygroundSmall
                | PlaygroundLarge
                | BaseballFieldSmall
                | SoccerFieldSmall
                | FootballField
                | SkatePark
                | MiniGolfCourse
                | BleachersField
                | GoKartTrack
                | Amphitheater
                | GreenhouseGarden
                | AnimalPensFarm
                | CabinHouse
                | Campground
                | MarinaDocksSmall
                | PierLarge
                | RollerCoasterSmall
                | CommunityGarden
                | PondPark
                | ParkGate
                | MountainLodge
                | MountainTrailhead
        )
    }

    pub fn is_service(self) -> bool {
        use BuildingType::*;
        matches!(
            self,
            PoliceStation | FireStation | Hospital | School | University | PowerPlant | WaterTower
        )
    }

    pub fn is_green_pollution_reducer(self) -> bool {
        use BuildingType::*;
        matches!(
            self,
            Tree | Park
                | ParkLarge
                | Tennis
                | BasketballCourts
                | PlaygroundSmall
                | PlaygroundLarge
                | BaseballFieldSmall
                | SoccerFieldSmall
                | FootballField
                | SwimmingPool
                | SkatePark
                | MiniGolfCourse
                | BleachersField
                | Amphitheater
                | GreenhouseGarden
                | CommunityGarden
                | PondPark
                | ParkGate
                | Campground
                | MountainTrailhead
        )
    }

    pub fn is_residential(self) -> bool {
        RESIDENTIAL_BUILDINGS.contains(&self)
    }

    pub fn is_commercial(self) -> bool {
        COMMERCIAL_BUILDINGS.contains(&self)
    }

    pub fn is_industrial(self) -> bool {
        INDUSTRIAL_BUILDINGS.contains(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_names_match_web_format() {
        assert_eq!(
            serde_json::to_string(&BuildingType::BaseballFieldSmall).unwrap(),
            "\"baseball_field_small\""
        );
        let t: BuildingType = serde_json::from_str("\"space_program\"").unwrap();
        assert_eq!(t, BuildingType::SpaceProgram);
    }

    #[test]
    fn sizes() {
        assert_eq!(BuildingType::Airport.size().width, 4);
        assert_eq!(BuildingType::HouseSmall.size().width, 1);
        assert!(BuildingType::Mall.size().is_multi());
    }
}
