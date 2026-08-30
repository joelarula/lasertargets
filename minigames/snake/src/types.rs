use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Unique game ID for Snake
pub const GAME_ID: u16 = 2;
pub const GAME_NAME: &str = "snake";

/// Grid cell size in world units (10cm)
pub const CELL_SIZE: f32 = 0.1;

/// Initial move interval in seconds (10 steps per second — smooth & controllable)
pub const INITIAL_TICK_INTERVAL: f32 = 0.10;

/// Minimum tick interval (top speed — 20 steps/sec)
pub const MIN_TICK_INTERVAL: f32 = 0.05;

/// Speed-up factor per gem eaten (gentle, smooth progression)
pub const SPEED_UP_PER_GEM: f32 = 0.002;

/// Size of a snake segment (as fraction of cell)
pub const SEGMENT_RADIUS: f32 = CELL_SIZE * 0.4;

/// Size of the diamond gem (half-diagonal) — huge & bold (2.5x cell size)
pub const GEM_HALF_SIZE: f32 = CELL_SIZE * 2.5;

// ---------------------------------------------------------------------------
// Direction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SnakeDirection {
    Up,
    Down,
    Left,
    Right,
}

impl SnakeDirection {
    /// Unit delta on the grid
    pub fn delta(self) -> IVec2 {
        match self {
            SnakeDirection::Up => IVec2::new(0, 1),
            SnakeDirection::Down => IVec2::new(0, -1),
            SnakeDirection::Left => IVec2::new(-1, 0),
            SnakeDirection::Right => IVec2::new(1, 0),
        }
    }

    /// Returns true if `self` is opposite to `other`
    pub fn is_opposite(self, other: SnakeDirection) -> bool {
        matches!(
            (self, other),
            (SnakeDirection::Up, SnakeDirection::Down)
                | (SnakeDirection::Down, SnakeDirection::Up)
                | (SnakeDirection::Left, SnakeDirection::Right)
                | (SnakeDirection::Right, SnakeDirection::Left)
        )
    }
}
