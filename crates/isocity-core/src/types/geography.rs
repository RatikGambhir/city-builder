//! Directions, neighbouring cities and water bodies.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];

    /// Grid step. North is `x - 1`, east is `y - 1` (isometric convention).
    pub fn step(self) -> (i32, i32) {
        match self {
            Direction::North => (-1, 0),
            Direction::East => (0, -1),
            Direction::South => (1, 0),
            Direction::West => (0, 1),
        }
    }

    pub fn opposite(self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }

    pub fn from_step(dx: i32, dy: i32) -> Option<Direction> {
        match (dx, dy) {
            (-1, 0) => Some(Direction::North),
            (0, -1) => Some(Direction::East),
            (1, 0) => Some(Direction::South),
            (0, 1) => Some(Direction::West),
            _ => None,
        }
    }

    pub fn is_ns(self) -> bool {
        matches!(self, Direction::North | Direction::South)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdjacentCity {
    pub id: String,
    pub name: String,
    pub direction: Direction,
    pub connected: bool,
    pub discovered: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaterBodyKind {
    Lake,
    Ocean,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaterBody {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: WaterBodyKind,
    pub tiles: Vec<Point>,
    pub center_x: f32,
    pub center_y: f32,
}
