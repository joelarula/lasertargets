use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::SnakeDirection;

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

/// Resource for terminal-side snake stats display
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnakeGameStats {
    pub session_id: Uuid,
    pub score: u32,
    pub length: u32,
    pub game_over: bool,
}
