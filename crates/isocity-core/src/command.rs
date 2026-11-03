//! Every change a player can make to a city is a [`Command`].
//!
//! Commands are plain serializable data, so a future online mode only needs to
//! ship them between peers (or to an authoritative server) and apply them in
//! the same order on every machine. Single-player applies them immediately via
//! [`crate::session::LocalSession`].

use crate::place;
use crate::tools::Tool;
use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Use `tool` on one tile (charges the tool's cost on success).
    ApplyTool { tool: Tool, x: i32, y: i32 },
    /// Finish a road/rail drag; spans water crossings on the path with bridges.
    FinishTrackDrag { path: Vec<(i32, i32)>, track: TrackType },
    UpgradeService { x: i32, y: i32 },
    SetTaxRate { rate: f32 },
    SetBudgetFunding { key: BudgetKey, funding: f32 },
    SetSpeed { speed: u8 },
    SetDisasters { enabled: bool },
    ConnectCity { id: String },
    ExpandCity,
    ShrinkCity,
    RenameCity { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    OutOfBounds,
    InsufficientFunds,
    NoChange,
    Invalid(&'static str),
}

pub type CommandResult = Result<(), Rejection>;

impl Command {
    /// Commands that only affect local presentation (not the shared city).
    pub fn is_local_only(&self) -> bool {
        matches!(self, Command::SetSpeed { .. })
    }
}

pub fn apply(state: &mut GameState, cmd: &Command) -> CommandResult {
    match cmd {
        Command::ApplyTool { tool, x, y } => apply_tool(state, *tool, *x, *y),
        Command::FinishTrackDrag { path, track } => {
            if place::create_bridges_on_path(state, path, *track) {
                Ok(())
            } else {
                Err(Rejection::NoChange)
            }
        }
        Command::UpgradeService { x, y } => {
            if place::upgrade_service(state, *x, *y) {
                Ok(())
            } else {
                Err(Rejection::Invalid("cannot upgrade"))
            }
        }
        Command::SetTaxRate { rate } => {
            state.tax_rate = rate.clamp(0.0, 100.0).round();
            Ok(())
        }
        Command::SetBudgetFunding { key, funding } => {
            state.budget.get_mut(*key).funding = funding.clamp(0.0, 100.0).round();
            Ok(())
        }
        Command::SetSpeed { speed } => {
            state.speed = (*speed).min(3);
            Ok(())
        }
        Command::SetDisasters { enabled } => {
            state.disasters_enabled = *enabled;
            Ok(())
        }
        Command::ConnectCity { id } => {
            let Some(city) = state.adjacent_cities.iter_mut().find(|c| &c.id == id) else {
                return Err(Rejection::Invalid("unknown city"));
            };
            if city.connected {
                return Err(Rejection::NoChange);
            }
            city.connected = true;
            city.discovered = true;
            let name = city.name.clone();
            state.stats.money += 5000;
            state.stats.income += 200;
            state.push_notification(
                "City Connected!",
                &format!("Trade route established with {name}. +$5000 bonus and +$200/month income."),
                "road",
            );
            Ok(())
        }
        Command::ExpandCity => {
            place::expand_grid(state, 15);
            Ok(())
        }
        Command::ShrinkCity => {
            if place::shrink_grid(state, 15) {
                Ok(())
            } else {
                Err(Rejection::Invalid("city too small"))
            }
        }
        Command::RenameCity { name } => {
            let n = name.trim();
            if n.is_empty() {
                return Err(Rejection::Invalid("empty name"));
            }
            state.city_name = n.to_string();
            Ok(())
        }
    }
}

fn apply_tool(state: &mut GameState, tool: Tool, x: i32, y: i32) -> CommandResult {
    if tool == Tool::Select {
        return Err(Rejection::NoChange);
    }
    let Some(tile) = state.grid.get(x, y) else {
        return Err(Rejection::OutOfBounds);
    };
    let cost = tool.cost();
    if cost > 0 && state.stats.money < cost {
        return Err(Rejection::InsufficientFunds);
    }
    if let Some(z) = tool.zone() {
        if tile.zone == z && z != ZoneType::None {
            return Err(Rejection::NoChange);
        }
    }
    if let Some(b) = tool.building() {
        if tile.building.kind == b {
            return Err(Rejection::NoChange);
        }
    }
    let changed = match tool {
        Tool::Subway => place::place_subway(state, x, y),
        Tool::ZoneWater => place::terraform_water(state, x, y),
        Tool::ZoneLand => place::terraform_land(state, x, y),
        Tool::Bulldoze => place::bulldoze(state, x, y),
        _ => {
            if let Some(z) = tool.zone() {
                place::place_building(state, x, y, None, Some(z))
            } else if let Some(b) = tool.building() {
                place::place_building(state, x, y, Some(b), None)
            } else {
                false
            }
        }
    };
    if !changed {
        return Err(Rejection::NoChange);
    }
    state.stats.money -= cost;
    discover_cities(state);
    Ok(())
}

/// Marks neighbouring cities as discovered once a road reaches their border.
pub fn discover_cities(state: &mut GameState) {
    let mut found = Vec::new();
    for c in &state.adjacent_cities {
        if !c.connected && !c.discovered && place::has_road_at_edge(&state.grid, c.direction) {
            found.push(c.id.clone());
        }
    }
    for id in found {
        if let Some(c) = state.adjacent_cities.iter_mut().find(|c| c.id == id) {
            c.discovered = true;
            let (name, dir) = (c.name.clone(), c.direction);
            let d = match dir {
                Direction::North => "north",
                Direction::East => "east",
                Direction::South => "south",
                Direction::West => "west",
            };
            state.push_notification(
                "City Discovered!",
                &format!("Your road has reached the {d} border! You can now connect to {name}."),
                "road",
            );
        }
    }
}
