//! The per-tick simulation step that ties everything together.

use crate::buildings::BuildingType;
use crate::types::*;
use super::advisors::generate_advisor_messages;
use super::footprint::{apply_building_footprint, can_spawn_multi_tile, find_building_origin, has_road_access};
use super::growth::{construction_speed, evolve_building, is_starter_building, zone_buildings, zone_demand};
use super::services::calculate_service_coverage;
use super::stats::{calculate_stats, update_budget_costs};

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
