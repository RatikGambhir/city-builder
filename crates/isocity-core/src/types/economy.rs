//! Demand, city stats and the budget.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Demand {
    pub residential: f32,
    pub commercial: f32,
    pub industrial: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub population: i64,
    pub jobs: i64,
    pub money: i64,
    pub income: i64,
    pub expenses: i64,
    pub happiness: f32,
    pub health: f32,
    pub education: f32,
    pub safety: f32,
    pub environment: f32,
    pub demand: Demand,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            population: 0,
            jobs: 0,
            money: 100_000,
            income: 0,
            expenses: 0,
            happiness: 50.0,
            health: 50.0,
            education: 50.0,
            safety: 50.0,
            environment: 75.0,
            demand: Demand {
                residential: 50.0,
                commercial: 30.0,
                industrial: 40.0,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BudgetCategory {
    pub name: String,
    pub funding: f32,
    pub cost: i64,
}

impl BudgetCategory {
    pub(super) fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            funding: 100.0,
            cost: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKey {
    Police,
    Fire,
    Health,
    Education,
    Transportation,
    Parks,
    Power,
    Water,
}

impl BudgetKey {
    pub const ALL: [BudgetKey; 8] = [
        BudgetKey::Police,
        BudgetKey::Fire,
        BudgetKey::Health,
        BudgetKey::Education,
        BudgetKey::Transportation,
        BudgetKey::Parks,
        BudgetKey::Power,
        BudgetKey::Water,
    ];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub police: BudgetCategory,
    pub fire: BudgetCategory,
    pub health: BudgetCategory,
    pub education: BudgetCategory,
    pub transportation: BudgetCategory,
    pub parks: BudgetCategory,
    pub power: BudgetCategory,
    pub water: BudgetCategory,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            police: BudgetCategory::new("Police"),
            fire: BudgetCategory::new("Fire"),
            health: BudgetCategory::new("Health"),
            education: BudgetCategory::new("Education"),
            transportation: BudgetCategory::new("Transportation"),
            parks: BudgetCategory::new("Parks"),
            power: BudgetCategory::new("Power"),
            water: BudgetCategory::new("Water"),
        }
    }
}

impl Budget {
    pub fn get(&self, key: BudgetKey) -> &BudgetCategory {
        match key {
            BudgetKey::Police => &self.police,
            BudgetKey::Fire => &self.fire,
            BudgetKey::Health => &self.health,
            BudgetKey::Education => &self.education,
            BudgetKey::Transportation => &self.transportation,
            BudgetKey::Parks => &self.parks,
            BudgetKey::Power => &self.power,
            BudgetKey::Water => &self.water,
        }
    }

    pub fn get_mut(&mut self, key: BudgetKey) -> &mut BudgetCategory {
        match key {
            BudgetKey::Police => &mut self.police,
            BudgetKey::Fire => &mut self.fire,
            BudgetKey::Health => &mut self.health,
            BudgetKey::Education => &mut self.education,
            BudgetKey::Transportation => &mut self.transportation,
            BudgetKey::Parks => &mut self.parks,
            BudgetKey::Power => &mut self.power,
            BudgetKey::Water => &mut self.water,
        }
    }

    pub fn total_expenses(&self) -> i64 {
        BudgetKey::ALL
            .iter()
            .map(|k| {
                let c = self.get(*k);
                (c.cost as f32 * c.funding / 100.0).floor() as i64
            })
            .sum()
    }
}
