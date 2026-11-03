//! The simulation tick: utilities and service coverage, zone growth and
//! densification, abandonment, pollution, fire, demand and the economy.

use crate::buildings::{
    BuildingType, Footprint, COMMERCIAL_BUILDINGS, INDUSTRIAL_BUILDINGS, RESIDENTIAL_BUILDINGS,
};
use crate::rng::Rng;
use crate::tools::Tool;
use crate::types::*;

pub const SERVICE_MAX_LEVEL: u8 = 5;
pub const SERVICE_RANGE_INCREASE_PER_LEVEL: f32 = 0.2;
pub const SERVICE_UPGRADE_COST_BASE: i64 = 2;

/// Base coverage radius (in tiles) for service buildings.
pub fn service_range(kind: BuildingType) -> Option<i32> {
    use BuildingType::*;
    Some(match kind {
        PoliceStation => 13,
        FireStation => 18,
        Hospital => 24,
        School => 11,
        University => 19,
        PowerPlant => 15,
        WaterTower => 12,
        _ => return None,
    })
}

/// Effective range after level upgrades (level 1: 100%, level 5: 180%).
pub fn effective_service_range(kind: BuildingType, level: u8) -> Option<i32> {
    service_range(kind).map(|r| {
        (r as f32 * (1.0 + (level.max(1) as f32 - 1.0) * SERVICE_RANGE_INCREASE_PER_LEVEL)).floor()
            as i32
    })
}

pub fn upgrade_cost(kind: BuildingType, level: u8) -> Option<i64> {
    let tool = Tool::ALL.iter().find(|t| t.building() == Some(kind))?;
    Some(tool.cost() * SERVICE_UPGRADE_COST_BASE.pow(level as u32))
}

/// Small houses, small shops and small factories/farms work without utilities.
pub fn is_starter_building(kind: BuildingType) -> bool {
    matches!(
        kind,
        BuildingType::HouseSmall | BuildingType::ShopSmall | BuildingType::FactorySmall
    )
}

pub fn calculate_service_coverage(grid: &Grid) -> ServiceCoverage {
    let size = grid.size();
    let mut services = ServiceCoverage::new(size);
    for t in grid.iter() {
        let kind = t.building.kind;
        if !kind.is_service() || t.building.under_construction() || t.building.abandoned {
            continue;
        }
        let Some(range) = effective_service_range(kind, t.building.level) else {
            continue;
        };
        let (x, y) = (t.x, t.y);
        let r2 = range * range;
        let (min_y, max_y) = ((y - range).max(0), (y + range).min(size - 1));
        let (min_x, max_x) = ((x - range).max(0), (x + range).min(size - 1));
        for ny in min_y..=max_y {
            for nx in min_x..=max_x {
                let (dx, dy) = (nx - x, ny - y);
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                let i = services.idx(nx, ny);
                match kind {
                    BuildingType::PowerPlant => services.power[i] = true,
                    BuildingType::WaterTower => services.water[i] = true,
                    _ => {
                        let cov = ((1.0 - (d2 as f32).sqrt() / range as f32) * 100.0).max(0.0);
                        let arr = match kind {
                            BuildingType::PoliceStation => &mut services.police,
                            BuildingType::FireStation => &mut services.fire,
                            BuildingType::Hospital => &mut services.health,
                            _ => &mut services.education,
                        };
                        arr[i] = (arr[i] + cov).min(100.0);
                    }
                }
            }
        }
    }
    services
}

/// BFS through same-zone tiles looking for a road within `max_distance`.
pub fn has_road_access(grid: &Grid, x: i32, y: i32, max_distance: i32) -> bool {
    let start_zone = grid.at(x, y).zone;
    if start_zone == ZoneType::None {
        return false;
    }
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back((x, y, 0));
    visited.insert((x, y));
    let mut budget = 256;
    while let Some((cx, cy, dist)) = queue.pop_front() {
        if dist >= max_distance {
            continue;
        }
        for (nx, ny) in [(cx - 1, cy), (cx + 1, cy), (cx, cy - 1), (cx, cy + 1)] {
            if !grid.in_bounds(nx, ny) || !visited.insert((nx, ny)) {
                continue;
            }
            let n = grid.at(nx, ny);
            if matches!(n.building.kind, BuildingType::Road | BuildingType::Bridge) {
                return true;
            }
            if n.zone == start_zone && n.building.kind != BuildingType::Water && budget > 0 {
                budget -= 1;
                queue.push_back((nx, ny, dist + 1));
            }
        }
    }
    false
}

fn can_spawn_multi_tile(grid: &Grid, x: i32, y: i32, fp: Footprint, zone: ZoneType) -> bool {
    if x + fp.width > grid.size() || y + fp.height > grid.size() {
        return false;
    }
    for dy in 0..fp.height {
        for dx in 0..fp.width {
            let t = grid.at(x + dx, y + dy);
            if t.zone != zone || !matches!(t.building.kind, BuildingType::Grass | BuildingType::Tree) {
                return false;
            }
        }
    }
    true
}

pub fn construction_speed(kind: BuildingType, rng: &mut Rng) -> f32 {
    let fp = kind.size();
    let area = (fp.width * fp.height) as f32;
    let base = 24.0 + rng.next_f32() * 12.0;
    (base / area.sqrt()) / 1.3
}

fn consolidatable(zone: ZoneType, kind: BuildingType) -> bool {
    use BuildingType::*;
    match zone {
        ZoneType::Residential => matches!(kind, HouseSmall | HouseMedium),
        ZoneType::Commercial => matches!(kind, ShopSmall | ShopMedium),
        ZoneType::Industrial => kind == FactorySmall,
        ZoneType::None => false,
    }
}

fn is_mergeable(t: &Tile, zone: ZoneType, exclude: (i32, i32), allow_consolidation: bool) -> bool {
    if (t.x, t.y) == exclude {
        return t.zone == zone
            && !t.building.on_fire
            && !matches!(t.building.kind, BuildingType::Water | BuildingType::Road);
    }
    if t.zone != zone || t.building.on_fire {
        return false;
    }
    match t.building.kind {
        BuildingType::Water | BuildingType::Road | BuildingType::Bridge => false,
        BuildingType::Grass | BuildingType::Tree => true,
        k => allow_consolidation && consolidatable(zone, k),
    }
}

fn find_footprint_including_tile(
    grid: &Grid,
    x: i32,
    y: i32,
    fp: Footprint,
    zone: ZoneType,
    allow_consolidation: bool,
) -> Option<(i32, i32)> {
    let size = grid.size();
    let mut best: Option<(i32, i32, f32)> = None;
    for oy in (y - (fp.height - 1))..=y {
        for ox in (x - (fp.width - 1))..=x {
            if ox < 0 || oy < 0 || ox + fp.width > size || oy + fp.height > size {
                continue;
            }
            let ok = (0..fp.height).all(|dy| {
                (0..fp.width).all(|dx| {
                    is_mergeable(grid.at(ox + dx, oy + dy), zone, (x, y), allow_consolidation)
                })
            });
            if !ok {
                continue;
            }
            // Prefer footprints touching roads.
            let mut road_score = 0;
            for dy in 0..fp.height {
                for dx in 0..fp.width {
                    for (ax, ay) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        if grid.is_road_or_bridge(ox + dx + ax, oy + dy + ay) {
                            road_score += 1;
                        }
                    }
                }
            }
            let score = road_score as f32 - (fp.width * fp.height) as f32 * 0.25;
            if best.is_none_or(|b| score > b.2) {
                best = Some((ox, oy, score));
            }
        }
    }
    best.map(|b| (b.0, b.1))
}

/// Writes a building and its `empty` placeholder tiles. Returns nothing; the
/// anchor building lives at `(ox, oy)`.
pub fn apply_building_footprint(
    grid: &mut Grid,
    ox: i32,
    oy: i32,
    kind: BuildingType,
    zone: ZoneType,
    level: u8,
    services: Option<&ServiceCoverage>,
) {
    let fp = kind.size();
    let pollution = kind.stats().pollution;
    for dy in 0..fp.height {
        for dx in 0..fp.width {
            let cell = grid.at_mut(ox + dx, oy + dy);
            if dx == 0 && dy == 0 {
                cell.building = Building::new(kind);
                cell.building.level = level;
                if let Some(s) = services {
                    let i = s.idx(ox, oy);
                    cell.building.powered = s.power[i];
                    cell.building.watered = s.water[i];
                }
                cell.pollution = pollution;
            } else {
                cell.building = Building::new(BuildingType::Empty);
                cell.building.level = 0;
                cell.pollution = 0.0;
            }
            cell.has_rail_overlay = false;
            cell.zone = zone;
        }
    }
}

/// Origin of the multi-tile building covering `(x, y)`, if any.
pub fn find_building_origin(grid: &Grid, x: i32, y: i32) -> Option<(i32, i32, BuildingType)> {
    let tile = grid.get(x, y)?;
    let kind = tile.building.kind;
    if !kind.is_terrain() {
        return if kind.size().is_multi() {
            Some((x, y, kind))
        } else {
            None
        };
    }
    if kind == BuildingType::Empty {
        for dy in 0..4 {
            for dx in 0..4 {
                let (cx, cy) = (x - dx, y - dy);
                if let Some(t) = grid.get(cx, cy) {
                    let k = t.building.kind;
                    if !k.is_terrain() {
                        let fp = k.size();
                        if x >= cx && x < cx + fp.width && y >= cy && y < cy + fp.height {
                            return Some((cx, cy, k));
                        }
                    }
                }
            }
        }
    }
    None
}

fn zone_demand(demand: &Demand, zone: ZoneType) -> f32 {
    match zone {
        ZoneType::Residential => demand.residential,
        ZoneType::Commercial => demand.commercial,
        ZoneType::Industrial => demand.industrial,
        ZoneType::None => 0.0,
    }
}

fn zone_buildings(zone: ZoneType) -> &'static [BuildingType] {
    match zone {
        ZoneType::Residential => &RESIDENTIAL_BUILDINGS,
        ZoneType::Commercial => &COMMERCIAL_BUILDINGS,
        ZoneType::Industrial => &INDUSTRIAL_BUILDINGS,
        ZoneType::None => &[],
    }
}

/// Evolves the zoned building at `(x, y)` in place.
fn evolve_building(
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

fn average(v: &[f32]) -> f32 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f32>() / v.len() as f32
    }
}

pub fn calculate_stats(
    grid: &Grid,
    budget: &Budget,
    tax_rate: f32,
    effective_tax_rate: f32,
    services: &ServiceCoverage,
) -> Stats {
    let size = grid.size();
    let mut population: i64 = 0;
    let mut jobs: i64 = 0;
    let mut total_pollution = 0.0f32;
    let (mut trees, mut water, mut parks) = (0, 0, 0);
    let (mut subway_tiles, mut subway_stations, mut rail_tiles, mut rail_stations) = (0, 0, 0, 0);
    let (mut airport, mut city_hall, mut space, mut amusement) = (false, false, false, false);
    let (mut stadiums, mut museums) = (0, 0);

    for t in grid.iter() {
        let b = &t.building;
        let mut tile_jobs = b.jobs as i64;
        if t.has_subway && t.zone == ZoneType::Commercial {
            tile_jobs = (tile_jobs as f64 * 1.15).floor() as i64;
        }
        population += b.population as i64;
        jobs += tile_jobs;
        total_pollution += t.pollution;
        match b.kind {
            BuildingType::Tree => trees += 1,
            BuildingType::Water => water += 1,
            BuildingType::Park | BuildingType::ParkLarge | BuildingType::Tennis => parks += 1,
            _ => {}
        }
        if t.has_subway {
            subway_tiles += 1;
        }
        if b.kind == BuildingType::SubwayStation {
            subway_stations += 1;
        }
        if b.kind == BuildingType::Rail || t.has_rail_overlay {
            rail_tiles += 1;
        }
        if b.kind == BuildingType::RailStation {
            rail_stations += 1;
        }
        if !b.under_construction() {
            match b.kind {
                BuildingType::Airport => airport = true,
                BuildingType::CityHall => city_hall = true,
                BuildingType::SpaceProgram => space = true,
                BuildingType::Stadium => stadiums += 1,
                BuildingType::Museum => museums += 1,
                BuildingType::AmusementPark => amusement = true,
                _ => {}
            }
        }
    }

    let (pop, jb) = (population as f32, jobs as f32);
    let tax_mult = (1.0 - (effective_tax_rate - 9.0) / 91.0).max(0.0);
    let tax_add = (9.0 - effective_tax_rate) * 2.0;
    let subway_bonus = (subway_tiles as f32 * 0.5 + subway_stations as f32 * 3.0).min(20.0);
    let rail_com = (rail_tiles as f32 * 0.15 + rail_stations as f32 * 4.0).min(12.0);
    let rail_ind = (rail_tiles as f32 * 0.25 + rail_stations as f32 * 6.0).min(18.0);
    let b = |c: bool, v: f32| if c { v } else { 0.0 };

    let base_res = (jb - pop * 0.7) / 18.0;
    let base_com = (pop * 0.3 - jb * 0.3) / 4.0 + subway_bonus;
    let base_ind = (pop * 0.35 - jb * 0.3) / 2.0;
    let res = base_res + b(city_hall, 8.0) + b(space, 10.0) + (museums as f32 * 5.0).min(10.0);
    let com = base_com
        + b(airport, 15.0)
        + b(city_hall, 10.0)
        + (stadiums as f32 * 12.0).min(20.0)
        + (museums as f32 * 8.0).min(15.0)
        + b(amusement, 18.0)
        + rail_com;
    let ind = base_ind + b(airport, 10.0) + b(city_hall, 5.0) + b(space, 20.0) + rail_ind;

    let income = (pop * tax_rate * 0.1 + jb * tax_rate * 0.05).floor() as i64;
    let expenses = budget.total_expenses();

    let n = (size * size).max(1) as f32;
    let police = average(&services.police);
    let fire = average(&services.fire);
    let health_cov = average(&services.health);
    let edu = average(&services.education);
    let safety = (police * 0.7 + fire * 0.3).min(100.0);
    let health = (health_cov * 0.8 + (100.0 - total_pollution / n) * 0.2).min(100.0);
    let education = edu.min(100.0);
    let green_ratio = (trees + water + parks) as f32 / n;
    let pollution_ratio = total_pollution / (n * 100.0);
    let environment = (green_ratio * 200.0 - pollution_ratio * 100.0 + 50.0).clamp(0.0, 100.0);
    let job_sat = if jb >= pop { 100.0 } else { jb / pop.max(1.0) * 100.0 };
    let happiness = (safety * 0.15
        + health * 0.2
        + education * 0.15
        + environment * 0.15
        + job_sat * 0.2
        + (100.0 - tax_rate * 3.0) * 0.15)
        .min(100.0);

    Stats {
        population,
        jobs,
        money: 0,
        income,
        expenses,
        happiness,
        health,
        education,
        safety,
        environment,
        demand: Demand {
            residential: (res * tax_mult + tax_add).clamp(-100.0, 100.0),
            commercial: (com * tax_mult + tax_add * 0.8).clamp(-100.0, 100.0),
            industrial: (ind * tax_mult + tax_add * 0.5).clamp(-100.0, 100.0),
        },
    }
}

pub fn update_budget_costs(grid: &Grid, budget: &mut Budget) {
    use BuildingType::*;
    let (mut police, mut fire, mut hospital, mut school, mut uni) = (0, 0, 0, 0, 0);
    let (mut park, mut power, mut water, mut road, mut subway_tiles, mut subway_st) = (0, 0, 0, 0, 0, 0);
    for t in grid.iter() {
        if t.has_subway {
            subway_tiles += 1;
        }
        match t.building.kind {
            PoliceStation => police += 1,
            FireStation => fire += 1,
            Hospital => hospital += 1,
            School => school += 1,
            University => uni += 1,
            Park | ParkLarge | Tennis => park += 1,
            PowerPlant => power += 1,
            WaterTower => water += 1,
            Road => road += 1,
            SubwayStation => subway_st += 1,
            _ => {}
        }
    }
    budget.police.cost = police * 50;
    budget.fire.cost = fire * 50;
    budget.health.cost = hospital * 100;
    budget.education.cost = school * 30 + uni * 100;
    budget.transportation.cost = road * 2 + subway_tiles * 3 + subway_st * 25;
    budget.parks.cost = park * 10;
    budget.power.cost = power * 150;
    budget.water.cost = water * 75;
}

pub fn generate_advisor_messages(stats: &Stats, grid: &Grid) -> Vec<AdvisorMessage> {
    let mut msgs = Vec::new();
    let (mut unpowered, mut unwatered) = (0, 0);
    let (mut abandoned, mut ab_r, mut ab_c, mut ab_i) = (0, 0, 0, 0);
    for t in grid.iter() {
        if t.zone != ZoneType::None && t.building.kind != BuildingType::Grass {
            if !t.building.powered {
                unpowered += 1;
            }
            if !t.building.watered {
                unwatered += 1;
            }
        }
        if t.building.abandoned {
            abandoned += 1;
            match t.zone {
                ZoneType::Residential => ab_r += 1,
                ZoneType::Commercial => ab_c += 1,
                ZoneType::Industrial => ab_i += 1,
                ZoneType::None => {}
            }
        }
    }
    let mut push = |name: &str, icon: &str, messages: Vec<String>, priority: Priority| {
        msgs.push(AdvisorMessage {
            name: name.into(),
            icon: icon.into(),
            messages,
            priority,
        })
    };
    if unpowered > 0 {
        push(
            "Power Advisor",
            "power",
            vec![format!("{unpowered} buildings lack power. Build more power plants!")],
            if unpowered > 10 { Priority::High } else { Priority::Medium },
        );
    }
    if unwatered > 0 {
        push(
            "Water Advisor",
            "water",
            vec![format!("{unwatered} buildings lack water. Build water towers!")],
            if unwatered > 10 { Priority::High } else { Priority::Medium },
        );
    }
    let net = stats.income - stats.expenses;
    if net < 0 {
        push(
            "Finance Advisor",
            "cash",
            vec![format!(
                "City is running a deficit of ${}/month. Consider raising taxes or cutting services.",
                net.abs()
            )],
            if net < -500 { Priority::Critical } else { Priority::High },
        );
    }
    if stats.safety < 40.0 {
        push(
            "Safety Advisor",
            "shield",
            vec!["Crime is on the rise. Build more police stations to protect citizens.".into()],
            if stats.safety < 20.0 { Priority::Critical } else { Priority::High },
        );
    }
    if stats.health < 50.0 {
        push(
            "Health Advisor",
            "hospital",
            vec!["Health services are lacking. Build hospitals to improve citizen health.".into()],
            if stats.health < 30.0 { Priority::High } else { Priority::Medium },
        );
    }
    if stats.education < 50.0 {
        push(
            "Education Advisor",
            "education",
            vec!["Education levels are low. Build schools and universities.".into()],
            if stats.education < 30.0 { Priority::High } else { Priority::Medium },
        );
    }
    if stats.environment < 40.0 {
        push(
            "Environment Advisor",
            "environment",
            vec!["Pollution is high. Plant trees and build parks to improve air quality.".into()],
            if stats.environment < 20.0 { Priority::High } else { Priority::Medium },
        );
    }
    let job_ratio = stats.jobs as f32 / (stats.population.max(1)) as f32;
    if stats.population > 100 && job_ratio < 0.8 {
        push(
            "Employment Advisor",
            "jobs",
            vec!["Unemployment is high. Zone more commercial and industrial areas.".into()],
            if job_ratio < 0.5 { Priority::High } else { Priority::Medium },
        );
    }
    if abandoned > 0 {
        let mut details = Vec::new();
        if ab_r > 0 {
            details.push(format!("{ab_r} residential"));
        }
        if ab_c > 0 {
            details.push(format!("{ab_c} commercial"));
        }
        if ab_i > 0 {
            details.push(format!("{ab_i} industrial"));
        }
        push(
            "Urban Planning Advisor",
            "planning",
            vec![
                format!(
                    "{abandoned} abandoned building{} in your city ({}).",
                    if abandoned > 1 { "s" } else { "" },
                    details.join(", ")
                ),
                "Oversupply has caused buildings to become vacant.".into(),
                "Increase demand by growing your city or wait for natural redevelopment.".into(),
            ],
            if abandoned > 10 {
                Priority::High
            } else if abandoned > 5 {
                Priority::Medium
            } else {
                Priority::Low
            },
        );
    }
    msgs
}

fn pollution_cleanup_profile(kind: BuildingType) -> Option<(i32, f32)> {
    if !kind.is_green_pollution_reducer() {
        return None;
    }
    let p = kind.stats().pollution;
    if p >= 0.0 {
        return None;
    }
    if kind == BuildingType::Tree {
        return Some((2, 2.0));
    }
    let fp = kind.size();
    let radius = fp.width.max(fp.height).max(2);
    let strength = (p.abs() * 0.35).round().clamp(2.0, 8.0);
    Some((radius, strength))
}

/// Advances the simulation by one tick (1/30 of a game day).
pub fn simulate_tick(state: &mut GameState) {
    let size = state.grid.size();
    let services = calculate_service_coverage(&state.grid);
    let demand = state.stats.demand;
    let mut rng = std::mem::take(&mut state.rng);
    let grid = &mut state.grid;
    let mut changed = false;

    for y in 0..size {
        for x in 0..size {
            let si = services.idx(x, y);
            let (new_powered, new_watered) = (services.power[si], services.water[si]);
            let (kind, zone, needs_update, pollution, construction, on_fire) = {
                let t = grid.at(x, y);
                let b = &t.building;
                (
                    b.kind,
                    t.zone,
                    b.powered != new_powered || b.watered != new_watered,
                    t.pollution,
                    b.construction_progress,
                    b.on_fire,
                )
            };
            if kind == BuildingType::Water {
                continue;
            }
            if matches!(kind, BuildingType::Road | BuildingType::Bridge) && !needs_update {
                continue;
            }
            if zone == ZoneType::None
                && matches!(kind, BuildingType::Grass | BuildingType::Tree)
                && !needs_update
                && pollution < 0.01
                && kind.stats().pollution == 0.0
            {
                continue;
            }
            let completed_service = zone == ZoneType::None
                && construction >= 100.0
                && !on_fire
                && !matches!(kind, BuildingType::Grass | BuildingType::Tree | BuildingType::Empty);
            if completed_service && !needs_update && pollution < 0.01 {
                continue;
            }

            {
                let b = &mut grid.at_mut(x, y).building;
                b.powered = new_powered;
                b.watered = new_watered;
            }

            // Construction for non-zoned buildings
            {
                let t = grid.at(x, y);
                if t.zone == ZoneType::None && t.building.under_construction() && t.building.kind.needs_construction() {
                    let utility = matches!(t.building.kind, BuildingType::PowerPlant | BuildingType::WaterTower);
                    if utility || (new_powered && new_watered) {
                        let speed = construction_speed(t.building.kind, &mut rng);
                        let b = &mut grid.at_mut(x, y).building;
                        let before = b.construction_progress;
                        b.construction_progress = (b.construction_progress + speed).min(100.0);
                        if (before < 40.0) != (b.construction_progress < 40.0) || b.construction_progress >= 100.0 {
                            changed = true;
                        }
                    }
                }
            }

            // Orphaned placeholders
            if grid.at(x, y).building.kind == BuildingType::Empty && find_building_origin(grid, x, y).is_none() {
                let b = &mut grid.at_mut(x, y).building;
                *b = Building::new(BuildingType::Grass);
                b.powered = new_powered;
                b.watered = new_watered;
                changed = true;
            }

            let (zone, kind) = {
                let t = grid.at(x, y);
                (t.zone, t.building.kind)
            };
            if zone != ZoneType::None && kind == BuildingType::Grass {
                let road = has_road_access(grid, x, y, 8);
                let zd = zone_demand(&demand, zone);
                let factor = ((zd + 30.0) / 80.0).clamp(0.0, 1.0);
                let spawn_chance = 0.05 * factor as f64;
                let candidate = zone_buildings(zone)[0];
                let starter = is_starter_building(candidate);
                if road && ((new_powered && new_watered) || starter) && rng.chance(spawn_chance) {
                    let fp = candidate.size();
                    if can_spawn_multi_tile(grid, x, y, fp, zone) {
                        apply_building_footprint(grid, x, y, candidate, zone, 1, Some(&services));
                        changed = true;
                    }
                }
            } else if zone != ZoneType::None && kind != BuildingType::Grass {
                let before = (kind, grid.at(x, y).building.level, grid.at(x, y).building.abandoned, grid.at(x, y).building.construction_progress < 40.0);
                evolve_building(grid, x, y, &services, &demand, &mut rng);
                let b = &grid.at(x, y).building;
                if before != (b.kind, b.level, b.abandoned, b.construction_progress < 40.0)
                    || (b.under_construction() != (before.3))
                {
                    changed = true;
                }
            }

            // Pollution
            {
                let t = grid.at_mut(x, y);
                let p = t.building.kind.stats().pollution;
                t.pollution = (t.pollution * 0.95 + p).max(0.0);
            }

            // Fire
            if state.disasters_enabled {
                let fire_cov = services.fire[si];
                if grid.at(x, y).building.on_fire {
                    if rng.chance((fire_cov / 300.0) as f64) {
                        let b = &mut grid.at_mut(x, y).building;
                        b.on_fire = false;
                        b.fire_progress = 0.0;
                        changed = true;
                    } else {
                        let t = grid.at_mut(x, y);
                        t.building.fire_progress += 2.0 / 3.0;
                        if t.building.fire_progress >= 100.0 {
                            t.building = Building::new(BuildingType::Grass);
                            t.zone = ZoneType::None;
                            changed = true;
                        }
                    }
                }
                let kind = grid.at(x, y).building.kind;
                let flammable = !grid.at(x, y).building.on_fire
                    && !matches!(
                        kind,
                        BuildingType::Grass
                            | BuildingType::Water
                            | BuildingType::Road
                            | BuildingType::Tree
                            | BuildingType::Empty
                            | BuildingType::Bridge
                            | BuildingType::Rail
                    );
                if flammable {
                    let adjacent = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                        .iter()
                        .filter(|(dx, dy)| {
                            grid.get(x + dx, y + dy).is_some_and(|n| n.building.on_fire)
                        })
                        .count();
                    if adjacent > 0 {
                        let spread = 0.005 * adjacent as f32 * (1.0 - fire_cov / 100.0 * 0.95);
                        if rng.chance(spread as f64) {
                            let b = &mut grid.at_mut(x, y).building;
                            b.on_fire = true;
                            b.fire_progress = 0.0;
                            changed = true;
                        }
                    }
                    if !grid.at(x, y).building.on_fire && rng.chance(0.00003) {
                        let b = &mut grid.at_mut(x, y).building;
                        b.on_fire = true;
                        b.fire_progress = 0.0;
                        changed = true;
                    }
                }
            }
        }
    }

    // Green amenities clean nearby pollution.
    for y in 0..size {
        for x in 0..size {
            let kind = grid.at(x, y).building.kind;
            let Some((radius, strength)) = pollution_cleanup_profile(kind) else {
                continue;
            };
            let fp = kind.size();
            let (x2, y2) = (x + fp.width - 1, y + fp.height - 1);
            for ty in (y - radius).max(0)..=(y2 + radius).min(size - 1) {
                for tx in (x - radius).max(0)..=(x2 + radius).min(size - 1) {
                    let dx = if tx < x { x - tx } else if tx > x2 { tx - x2 } else { 0 };
                    let dy = if ty < y { y - ty } else if ty > y2 { ty - y2 } else { 0 };
                    let d = dx + dy;
                    if d > radius {
                        continue;
                    }
                    let amount = strength * (1.0 - d as f32 / (radius + 1) as f32);
                    if amount > 0.0 {
                        let t = grid.at_mut(tx, ty);
                        t.pollution = (t.pollution - amount).max(0.0);
                    }
                }
            }
        }
    }

    update_budget_costs(&state.grid, &mut state.budget);
    let eff = state.effective_tax_rate();
    let new_eff = eff + (state.tax_rate - eff) * 0.03;
    state.effective_tax_rate = Some(new_eff);

    let mut stats = calculate_stats(&state.grid, &state.budget, state.tax_rate, new_eff, &services);
    stats.money = state.stats.money;
    let prev = state.stats.demand;
    let s = 0.12;
    stats.demand.residential = prev.residential + (stats.demand.residential - prev.residential) * s;
    stats.demand.commercial = prev.commercial + (stats.demand.commercial - prev.commercial) * s;
    stats.demand.industrial = prev.industrial + (stats.demand.industrial - prev.industrial) * s;
    // Trade income from connected neighbouring cities.
    let connected = state.adjacent_cities.iter().filter(|c| c.connected).count() as i64;
    stats.income += connected * 200;

    // Calendar
    let mut tick = state.tick + 1;
    let total_ticks = (state.year - 2024) * 12 * 30 * 30
        + (state.month - 1) * 30 * 30
        + (state.day - 1) * 30
        + tick;
    let cycle = 450;
    state.hour = (((total_ticks.rem_euclid(cycle)) as f32 / cycle as f32) * 24.0).floor() as i32;
    if tick >= 30 {
        tick = 0;
        state.day += 1;
        if state.day % 7 == 0 {
            stats.money += ((stats.income - stats.expenses) as f64 / 4.0).floor() as i64;
        }
    }
    if state.day > 30 {
        state.day = 1;
        state.month += 1;
    }
    if state.month > 12 {
        state.month = 1;
        state.year += 1;
    }
    state.tick = tick;

    state.advisor_messages = generate_advisor_messages(&stats, &state.grid);
    if state.month % 3 == 0 && state.day == 1 && tick == 0 {
        state.history.push(HistoryPoint {
            year: state.year,
            month: state.month,
            population: stats.population,
            money: stats.money,
            happiness: stats.happiness,
        });
        if state.history.len() > 100 {
            state.history.remove(0);
        }
    }
    state.stats = stats;
    state.services = services;
    state.rng = rng;
    if changed {
        state.touch();
    }
}
