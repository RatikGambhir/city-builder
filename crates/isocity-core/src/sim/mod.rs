//! The simulation tick: utilities and service coverage, zone growth and
//! densification, abandonment, pollution, fire, demand and the economy.

mod services;
mod footprint;
mod growth;
mod stats;
mod advisors;
mod tick;

pub use services::{SERVICE_MAX_LEVEL, SERVICE_RANGE_INCREASE_PER_LEVEL, SERVICE_UPGRADE_COST_BASE, calculate_service_coverage, effective_service_range, service_range, upgrade_cost};
pub use footprint::{apply_building_footprint, find_building_origin, has_road_access};
pub use growth::{construction_speed, is_starter_building};
pub use stats::{calculate_stats, update_budget_costs};
pub use advisors::generate_advisor_messages;
pub use tick::simulate_tick;

