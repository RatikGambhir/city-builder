//! Sprite pack configuration: which sheet, cell and offsets each building uses.
//!
//! This is a direct port of IsoCity's `renderConfig.ts` / `buildingSprite.ts`
//! so buildings sit on their tiles exactly like in the web version.

mod main_sheet;
mod variants;
mod parks;
mod services;
mod placement;

pub use main_sheet::sprite_key;
pub use parks::parks_cell;
pub use placement::{crop_for, offsets_for, scale_for};

use isocity_core::{Building, BuildingType as B};
use main_sheet::main_cell;
use services::services_row;
use variants::{FARM_VARIANTS, MANSION_VARIANTS, STATION_VARIANTS, dense_variants, modern_variants, shop_variants};

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
