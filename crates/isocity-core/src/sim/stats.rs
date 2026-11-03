//! City-wide statistics and running budget costs.

use crate::buildings::BuildingType;
use crate::types::*;

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
