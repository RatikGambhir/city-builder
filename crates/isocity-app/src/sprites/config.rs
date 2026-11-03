//! Sprite pack configuration: which sheet, cell and offsets each building uses.
//!
//! This is a direct port of IsoCity's `renderConfig.ts` / `buildingSprite.ts`
//! so buildings sit on their tiles exactly like in the web version.

use isocity_core::{Building, BuildingType as B};

/// Every sprite sheet the game knows about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sheet {
    Main,
    Construction,
    Abandoned,
    Dense,
    Modern,
    Parks,
    ParksConstruction,
    Farms,
    Shops,
    Stations,
    Services,
    Infrastructure,
    Mansions,
    Planes,
    HarryMain,
    HarryDense,
    HarryConstruction,
    ChinaMain,
}

impl Sheet {
    pub fn file(self) -> &'static str {
        match self {
            Sheet::Main => "sprites_red_water_new.webp",
            Sheet::Construction => "sprites_red_water_new_construction.webp",
            Sheet::Abandoned => "sprites_red_water_new_abandoned.webp",
            Sheet::Dense => "sprites_red_water_new_dense.webp",
            Sheet::Modern => "sprites_red_water_new_modern.webp",
            Sheet::Parks => "sprites_red_water_new_parks.webp",
            Sheet::ParksConstruction => "sprites_red_water_new_parks_construction.webp",
            Sheet::Farms => "sprites_red_water_new_farm.webp",
            Sheet::Shops => "sprites_red_water_new_shops.webp",
            Sheet::Stations => "sprites_red_water_new_stations.webp",
            Sheet::Services => "sprites_red_water_new_services.webp",
            Sheet::Infrastructure => "sprites_red_water_new_services-2.webp",
            Sheet::Mansions => "mansion_alternates.webp",
            Sheet::Planes => "sprites_red_water_new_planes.webp",
            Sheet::HarryMain => "sprites_red_water_new_harry.webp",
            Sheet::HarryDense => "sprites_red_water_new_harry_dense.webp",
            Sheet::HarryConstruction => "sprites_red_water_new_harry_construction.webp",
            Sheet::ChinaMain => "sprites_red_water_new_china.webp",
        }
    }

    /// Grid layout (cols, rows) of the sheet.
    pub fn layout(self) -> (u32, u32) {
        match self {
            Sheet::Mansions => (5, 7),
            _ => (5, 6),
        }
    }
}

/// Visual theme ("sprite pack").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SpritePack {
    #[default]
    Default,
    Harry,
    China,
}

impl SpritePack {
    pub const ALL: [SpritePack; 3] = [SpritePack::Default, SpritePack::Harry, SpritePack::China];

    pub fn name(self) -> &'static str {
        match self {
            SpritePack::Default => "Default Theme",
            SpritePack::Harry => "Harry Potter Theme",
            SpritePack::China => "Chinese Theme",
        }
    }

    fn main(self) -> Sheet {
        match self {
            SpritePack::Default => Sheet::Main,
            SpritePack::Harry => Sheet::HarryMain,
            SpritePack::China => Sheet::ChinaMain,
        }
    }
    fn dense(self) -> Sheet {
        match self {
            SpritePack::Harry => Sheet::HarryDense,
            _ => Sheet::Dense,
        }
    }
    fn modern(self) -> Sheet {
        match self {
            SpritePack::Harry => Sheet::HarryDense,
            _ => Sheet::Modern,
        }
    }
    fn construction(self) -> Sheet {
        match self {
            SpritePack::Harry => Sheet::HarryConstruction,
            _ => Sheet::Construction,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variant {
    Normal,
    Construction,
    Abandoned,
    Parks,
    ParksConstruction,
    Dense,
    Modern,
    Farm,
    Shop,
    Station,
    Services,
    Infrastructure,
    Mansion,
}

/// Which sheet + cell a building uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SpriteSource {
    pub sheet: Sheet,
    pub variant: Variant,
    pub row: u32,
    pub col: u32,
}

/// Fractional source rectangle within a sheet, in cell units (multiplied by
/// the cell size when cropping).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellCrop {
    pub row: u32,
    pub col: u32,
    /// Extra offset (in cell heights) added to the top of the crop.
    pub dy: f32,
    /// Crop height in cell heights.
    pub h: f32,
}

type RC = (u32, u32);

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

fn main_cell(key: &str) -> Option<RC> {
    let i = SPRITE_ORDER.iter().position(|k| *k == key)? as u32;
    Some((i / 5, i % 5))
}

fn vertical_offset(key: &str) -> f32 {
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

fn horizontal_offset(key: &str) -> f32 {
    match key {
        "city_hall" => 0.1,
        _ => 0.0,
    }
}

fn building_vertical_offset(b: B) -> Option<f32> {
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

fn construction_vertical_offset(b: B) -> Option<f32> {
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

fn construction_scale(b: B) -> Option<f32> {
    Some(match b {
        B::Mall => 0.92,
        B::OfficeHigh => 0.80,
        B::ApartmentHigh => 0.65,
        B::ApartmentLow => 0.80,
        _ => return None,
    })
}

fn abandoned_vertical_offset(b: B) -> Option<f32> {
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

fn dense_variants(b: B) -> &'static [RC] {
    match b {
        B::ApartmentHigh => &[(0, 1), (0, 2), (0, 3)],
        B::OfficeLow => &[(3, 0), (3, 1), (3, 3)],
        B::OfficeHigh => &[(1, 0), (2, 0), (2, 1), (2, 2)],
        B::Mall => &[(1, 4), (2, 3), (2, 4), (3, 0), (3, 1), (3, 2), (3, 4)],
        B::FactoryLarge => &[(4, 0), (4, 2), (4, 4)],
        _ => &[],
    }
}

fn modern_variants(b: B) -> &'static [RC] {
    match b {
        B::ApartmentHigh => &[(0, 0), (0, 1)],
        B::Mall => &[(2, 0), (3, 0), (3, 3), (3, 4)],
        _ => &[],
    }
}

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

fn parks_vertical_offset(b: B) -> f32 {
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

fn parks_scale(b: B) -> f32 {
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

fn parks_construction_vertical_offset(b: B) -> Option<f32> {
    match b {
        B::BaseballFieldSmall | B::MountainLodge => Some(-0.55),
        _ => None,
    }
}

const FARM_VARIANTS: [RC; 15] = [
    (0, 0), (0, 1), (0, 2), (0, 3), (0, 4),
    (1, 0), (1, 1), (1, 2), (1, 3), (1, 4),
    (5, 0), (5, 1), (5, 2), (5, 3), (5, 4),
];

fn shop_variants(b: B) -> &'static [RC] {
    match b {
        B::ShopSmall => &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (1, 1), (1, 2), (1, 3), (1, 4), (3, 0), (3, 1)],
        B::ShopMedium => &[(3, 2), (3, 4), (4, 0), (4, 1), (4, 2), (4, 3), (4, 4), (5, 0), (5, 2)],
        _ => &[],
    }
}

const STATION_VARIANTS: [RC; 10] = [
    (2, 0), (2, 1), (2, 2), (3, 2), (3, 3), (4, 1), (4, 2), (4, 3), (5, 0), (5, 1),
];

fn services_row(b: B) -> Option<u32> {
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

fn services_vertical_offset(b: B) -> f32 {
    match b {
        B::PoliceStation | B::FireStation | B::School | B::PowerPlant => -0.3,
        B::Hospital => -0.50,
        B::University => -0.85,
        _ => 0.0,
    }
}

const MANSION_VARIANTS: [RC; 5] = [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)];

fn seed_pick(x: i32, y: i32, len: usize) -> usize {
    // (x * 31 + y * 17) % (len + 1); 0 means "use the base sprite".
    ((x * 31 + y * 17).rem_euclid(len as i32 + 1)) as usize
}

pub fn is_construction_phase(b: &Building) -> bool {
    b.construction_progress >= 40.0 && b.construction_progress < 100.0
}

pub fn is_foundation_phase(b: &Building) -> bool {
    b.construction_progress < 40.0
}

/// Chooses the sheet/cell for a building (priority order matches the web app).
pub fn select_source(pack: SpritePack, b: &Building, x: i32, y: i32) -> Option<SpriteSource> {
    let kind = b.kind;
    let construction = is_construction_phase(b) && b.kind.needs_construction();
    let park = parks_cell(kind);
    let src = |sheet, variant, (row, col): RC| SpriteSource { sheet, variant, row, col };

    if construction {
        if let Some(rc) = park {
            return Some(src(Sheet::ParksConstruction, Variant::ParksConstruction, rc));
        }
        if let Some(rc) = main_cell(sprite_key(kind)?) {
            return Some(src(pack.construction(), Variant::Construction, rc));
        }
    }
    if b.abandoned {
        if let Some(rc) = sprite_key(kind).and_then(main_cell) {
            return Some(src(Sheet::Abandoned, Variant::Abandoned, rc));
        }
    }
    if let Some(rc) = park {
        return Some(src(Sheet::Parks, Variant::Parks, rc));
    }
    let dense = dense_variants(kind);
    if !dense.is_empty() {
        let modern = modern_variants(kind);
        let total = dense.len() + modern.len();
        let s = seed_pick(x, y, total);
        if s > 0 {
            let i = s - 1;
            return Some(if i < dense.len() {
                src(pack.dense(), Variant::Dense, dense[i])
            } else {
                src(pack.modern(), Variant::Modern, modern[i - dense.len()])
            });
        }
    }
    if kind == B::FactorySmall {
        let s = seed_pick(x, y, FARM_VARIANTS.len());
        if s > 0 {
            return Some(src(Sheet::Farms, Variant::Farm, FARM_VARIANTS[s - 1]));
        }
    }
    let shops = shop_variants(kind);
    if !shops.is_empty() {
        let s = seed_pick(x, y, shops.len());
        if s > 0 {
            return Some(src(Sheet::Shops, Variant::Shop, shops[s - 1]));
        }
    }
    if kind == B::RailStation {
        let i = ((x * 7 + y * 13).rem_euclid(STATION_VARIANTS.len() as i32)) as usize;
        return Some(src(Sheet::Stations, Variant::Station, STATION_VARIANTS[i]));
    }
    if let Some(row) = services_row(kind) {
        let col = (b.level.max(1) as u32 - 1).min(4);
        return Some(src(Sheet::Services, Variant::Services, (row, col)));
    }
    if kind == B::WaterTower {
        let col = (b.level.max(1) as u32 - 1).min(4);
        return Some(src(Sheet::Infrastructure, Variant::Infrastructure, (0, col)));
    }
    if kind == B::Mansion {
        let s = seed_pick(x, y, MANSION_VARIANTS.len());
        if s > 0 {
            return Some(src(Sheet::Mansions, Variant::Mansion, MANSION_VARIANTS[s - 1]));
        }
    }
    let rc = main_cell(sprite_key(kind)?)?;
    Some(src(pack.main(), Variant::Normal, rc))
}

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
