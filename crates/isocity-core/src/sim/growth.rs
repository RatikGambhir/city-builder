//! Zone growth: what each zone builds, construction speed and how buildings
//! evolve, densify or get abandoned.

use crate::buildings::{BuildingType, COMMERCIAL_BUILDINGS, INDUSTRIAL_BUILDINGS, RESIDENTIAL_BUILDINGS};
use crate::rng::Rng;
use crate::types::*;
use super::footprint::{apply_building_footprint, find_footprint_including_tile};

/// Small houses, small shops and small factories/farms work without utilities.
pub fn is_starter_building(kind: BuildingType) -> bool {
    matches!(
        kind,
        BuildingType::HouseSmall | BuildingType::ShopSmall | BuildingType::FactorySmall
    )
}

pub fn construction_speed(kind: BuildingType, rng: &mut Rng) -> f32 {
    let fp = kind.size();
    let area = (fp.width * fp.height) as f32;
    let base = 24.0 + rng.next_f32() * 12.0;
    (base / area.sqrt()) / 1.3
}

pub(super) fn zone_demand(demand: &Demand, zone: ZoneType) -> f32 {
    match zone {
        ZoneType::Residential => demand.residential,
        ZoneType::Commercial => demand.commercial,
        ZoneType::Industrial => demand.industrial,
        ZoneType::None => 0.0,
    }
}

pub(super) fn zone_buildings(zone: ZoneType) -> &'static [BuildingType] {
    match zone {
        ZoneType::Residential => &RESIDENTIAL_BUILDINGS,
        ZoneType::Commercial => &COMMERCIAL_BUILDINGS,
        ZoneType::Industrial => &INDUSTRIAL_BUILDINGS,
        ZoneType::None => &[],
    }
}

/// Evolves the zoned building at `(x, y)` in place.
pub(super) fn evolve_building(
    grid: &mut Grid,
    x: i32,
    y: i32,
    services: &ServiceCoverage,
    demand: &Demand,
    rng: &mut Rng,
) {
    let si = services.idx(x, y);
    let (zone, kind, land_value) = {
        let t = grid.at(x, y);
        (t.zone, t.building.kind, t.land_value)
    };
    if zone == ZoneType::None
        || matches!(
            kind,
            BuildingType::Grass | BuildingType::Water | BuildingType::Road | BuildingType::Bridge
        )
    {
        return;
    }
    {
        let b = &mut grid.at_mut(x, y).building;
        b.powered = services.power[si];
        b.watered = services.water[si];
        if kind == BuildingType::Empty {
            b.population = 0;
            b.jobs = 0;
            return;
        }
    }
    let (has_power, has_water) = (services.power[si], services.water[si]);
    let is_starter = is_starter_building(kind);
    if !is_starter && (!has_power || !has_water) {
        return;
    }

    // Construction
    if grid.at(x, y).building.under_construction() {
        let speed = construction_speed(kind, rng);
        let b = &mut grid.at_mut(x, y).building;
        b.construction_progress = (b.construction_progress + speed).min(100.0);
        b.population = 0;
        b.jobs = 0;
        return;
    }

    let zd = zone_demand(demand, zone);

    // Abandonment & recovery
    if grid.at(x, y).building.abandoned {
        if zd > 10.0 {
            let clearing_chance = ((zd - 10.0) / 600.0).min(0.12) as f64;
            if rng.chance(clearing_chance) {
                let fp = kind.size();
                for dy in 0..fp.height {
                    for dx in 0..fp.width {
                        if let Some(t) = grid.get_mut(x + dx, y + dy) {
                            let mut b = Building::new(BuildingType::Grass);
                            let i = services.idx(t.x, t.y);
                            b.powered = services.power[i];
                            b.watered = services.water[i];
                            t.building = b;
                        }
                    }
                }
                return;
            }
        }
        let b = &mut grid.at_mut(x, y).building;
        b.population = 0;
        b.jobs = 0;
        b.age += 0.1;
        return;
    }

    {
        let b = &grid.at(x, y).building;
        if zd < -20.0 && b.age > 30.0 {
            let base = ((zd + 20.0).abs() / 4000.0).min(0.02);
            let utility = if is_starter {
                0.0
            } else {
                (if !has_power { 0.005 } else { 0.0 }) + (if !has_water { 0.005 } else { 0.0 })
            };
            let level_pen = if b.level <= 2 { 0.003 } else { 0.0 };
            if rng.chance((base + utility + level_pen) as f64) {
                let b = &mut grid.at_mut(x, y).building;
                b.abandoned = true;
                b.population = 0;
                b.jobs = 0;
                return;
            }
        }
    }

    grid.at_mut(x, y).building.age += 1.0;
    let (age, level) = {
        let b = &grid.at(x, y).building;
        (b.age, b.level)
    };

    let list = zone_buildings(zone);
    let service_cov = (services.police[si] + services.fire[si] + services.health[si] + services.education[si]) / 4.0;
    let demand_boost = ((zd - 30.0) / 70.0).max(0.0) * 0.7;
    let target_level = ((land_value / 24.0) + (service_cov / 28.0) + (age / 60.0) + demand_boost)
        .floor()
        .clamp(1.0, 5.0) as u8;
    let target_type = list[(target_level as usize - 1).min(list.len() - 1)];

    let mut consolidation_chance = 0.08;
    let mut allow_consolidation = false;
    let is_small = match zone {
        ZoneType::Residential => matches!(kind, BuildingType::HouseSmall | BuildingType::HouseMedium),
        ZoneType::Commercial => matches!(kind, BuildingType::ShopSmall | BuildingType::ShopMedium),
        ZoneType::Industrial => kind == BuildingType::FactorySmall,
        ZoneType::None => false,
    };
    if zd > 30.0 && is_small {
        consolidation_chance += ((zd - 30.0) / 300.0).min(0.25) as f64;
        if zd > 70.0 {
            consolidation_chance += 0.05;
            allow_consolidation = true;
        }
    }

    let (mut ax, mut ay) = (x, y);
    if has_power
        && has_water
        && age > 12.0
        && (target_level > level || target_type != kind)
        && rng.chance(consolidation_chance)
    {
        let fp = target_type.size();
        if let Some((ox, oy)) = find_footprint_including_tile(grid, x, y, fp, zone, allow_consolidation) {
            apply_building_footprint(grid, ox, oy, target_type, zone, target_level, Some(services));
            ax = ox;
            ay = oy;
        } else if target_level > level {
            grid.at_mut(x, y).building.level = target_level.min(level + 1);
        }
    }

    let ai = services.idx(ax, ay);
    let b = &mut grid.at_mut(ax, ay).building;
    b.powered = services.power[ai];
    b.watered = services.water[ai];
    b.level = b.level.max(target_level.min(b.level + 1));
    let stats = b.kind.stats();
    let efficiency = (if b.powered { 0.5 } else { 0.0 }) + (if b.watered { 0.5 } else { 0.0 });
    let lvl = b.level.max(1) as f32;
    b.population = if stats.max_pop > 0 {
        (stats.max_pop as f32 * lvl * efficiency * 0.8).floor() as u32
    } else {
        0
    };
    b.jobs = if stats.max_jobs > 0 {
        (stats.max_jobs as f32 * lvl * efficiency * 0.8).floor() as u32
    } else {
        0
    };
}
