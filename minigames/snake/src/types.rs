use bevy::prelude::*;
use common::calibration::ControlKey;
use serde::{Deserialize, Serialize};

/// Unique game ID for Snake
pub const GAME_ID: u16 = 2;
pub const GAME_NAME: &str = "snake";

/// Grid cell size in world units (10cm)
pub const CELL_SIZE: f32 = 0.1;

/// Cells the snake has at the start of a round
pub const START_LENGTH: i32 = 3;

/// Cells the snake grows by for every gem
pub const GROWTH_PER_GEM: usize = 2;

/// Initial move interval in seconds (10 steps per second — smooth & controllable)
pub const INITIAL_TICK_INTERVAL: f32 = 0.10;

/// Minimum tick interval (top speed — 20 steps/sec)
pub const MIN_TICK_INTERVAL: f32 = 0.05;

/// Speed-up factor per gem eaten (gentle, smooth progression)
pub const SPEED_UP_PER_GEM: f32 = 0.002;

/// How long GAME OVER shows before the next round starts
pub const RESTART_DELAY_SECS: f32 = 2.5;

/// Size of a snake segment (as fraction of cell)
pub const SEGMENT_RADIUS: f32 = CELL_SIZE * 0.4;

/// Size of the diamond gem (half-diagonal) — huge & bold (2.5x cell size)
pub const GEM_HALF_SIZE: f32 = CELL_SIZE * 2.5;

/// Gem colours; the body takes on the colour of the last gem eaten
pub const GEM_COLORS: [Color; 3] = [
    Color::srgb(1.0, 0.0, 0.0),
    Color::srgb(0.0, 1.0, 0.0),
    Color::srgb(0.0, 0.3, 1.0),
];

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
            Self::Up => IVec2::Y,
            Self::Down => IVec2::NEG_Y,
            Self::Left => IVec2::NEG_X,
            Self::Right => IVec2::X,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    /// Arrow keys steer; other keys don't
    pub fn from_key(key: ControlKey) -> Option<Self> {
        match key {
            ControlKey::Up => Some(Self::Up),
            ControlKey::Down => Some(Self::Down),
            ControlKey::Left => Some(Self::Left),
            ControlKey::Right => Some(Self::Right),
            _ => None,
        }
    }
}
