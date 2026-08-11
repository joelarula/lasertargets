use bevy::asset::uuid::Uuid;
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

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marker: the head entity of the snake
#[derive(Component, Debug, Clone)]
pub struct SnakeHead;

/// Marker: a body segment entity
#[derive(Component, Debug, Clone)]
pub struct SnakeSegment {
    /// The color this segment was given when the gem was eaten
    pub color: Color,
}

/// Marker: the static play-field border rectangle (never moved or respawned per tick)
#[derive(Component, Debug, Clone)]
pub struct SnakeBorder;

/// Marker: the diamond / gem food entity
#[derive(Component, Debug, Clone)]
pub struct DiamondFood {
    pub color: Color,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Authoritative snake state kept on server
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct SnakeState {
    /// Grid positions of all segments: index 0 = head
    pub segments: Vec<IVec2>,
    /// Colors of each segment (index 0 = head → white)
    pub segment_colors: Vec<(f32, f32, f32)>,
    /// Current movement direction
    pub direction: SnakeDirection,
    /// Queued direction change (applied on next tick)
    pub queued_direction: Option<SnakeDirection>,
    /// Grid position of the current gem
    pub gem_position: IVec2,
    /// Color of the current gem (r, g, b)
    pub gem_color: (f32, f32, f32),
    /// Number of gems eaten (= score)
    pub gems_eaten: u32,
    /// Number of pending growth segments to add on upcoming ticks
    pub pending_growth: usize,
    /// Grid width in cells
    pub grid_w: i32,
    /// Grid height in cells
    pub grid_h: i32,
    /// Session id
    pub session_id: Uuid,
    /// Whether the player has started moving
    pub is_started: bool,
    /// Whether the game is over
    pub game_over: bool,
    /// Auto-reset timer after Game Over screen
    #[serde(skip)]
    pub game_over_reset_timer: Option<Timer>,
}

/// Timer that drives snake movement ticks (server-side)
#[derive(Resource, Debug, Clone)]
pub struct SnakeMoveTimer {
    pub timer: Timer,
}

impl SnakeMoveTimer {
    pub fn new(interval: f32) -> Self {
        Self {
            timer: Timer::from_seconds(interval, TimerMode::Repeating),
        }
    }
}

// ---------------------------------------------------------------------------
// Events / Messages
// ---------------------------------------------------------------------------

/// Raised on server when direction should change (from keyboard input)
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSnakeDirectionEvent {
    pub direction: SnakeDirection,
}

/// Raised on server to broadcast stats to terminal
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastSnakeStatsEvent {
    pub session_id: Uuid,
    pub score: u32,
    pub length: u32,
    pub game_over: bool,
}

/// Raised on server when game ends
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct SnakeGameOverEvent {
    pub session_id: Uuid,
    pub final_score: u32,
}

/// Resource for terminal-side snake stats display
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnakeGameStats {
    pub session_id: Uuid,
    pub score: u32,
    pub length: u32,
    pub game_over: bool,
}
