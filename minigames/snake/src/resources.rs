use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::types::RESTART_DELAY_SECS;

/// Time between snake moves (server-side); shortened as the snake speeds up
#[derive(Resource, Debug, Clone, Deref, DerefMut)]
pub struct SnakeMoveTimer(pub Timer);

impl SnakeMoveTimer {
    pub fn new(interval: f32) -> Self {
        Self(Timer::from_seconds(interval, TimerMode::Repeating))
    }
}

/// How long GAME OVER shows before the next round (server-side)
#[derive(Resource, Debug, Clone, Deref, DerefMut)]
pub struct SnakeRestartTimer(pub Timer);

impl Default for SnakeRestartTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(RESTART_DELAY_SECS, TimerMode::Once))
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
