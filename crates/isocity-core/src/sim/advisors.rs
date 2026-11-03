//! Advisor messages derived from the current stats.

use crate::buildings::BuildingType;
use crate::types::*;

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
