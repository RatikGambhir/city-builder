//! Notifications, advisor messages and history samples.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon: String,
    #[serde(default)]
    pub timestamp: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdvisorMessage {
    pub name: String,
    pub icon: String,
    pub messages: Vec<String>,
    pub priority: Priority,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub year: i32,
    pub month: i32,
    pub population: i64,
    pub money: i64,
    pub happiness: f32,
}
