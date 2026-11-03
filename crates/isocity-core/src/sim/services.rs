//! Service buildings: coverage ranges, upgrade levels and the coverage map.

use crate::buildings::BuildingType;
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
