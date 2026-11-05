use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::curator_core::CuratorPayload;
use serde::{Deserialize, Serialize};

use crate::types::SnakeDirection;

/// Raised on server when direction should change (from keyboard/gamepad input)
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

impl CuratorPayload for BroadcastSnakeStatsEvent {
    fn event_tag() -> &'static str {
        "snake_stats"
    }
}

/// Raised on server when game ends
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct SnakeGameOverEvent {
    pub session_id: Uuid,
    pub final_score: u32,
}

impl CuratorPayload for SnakeGameOverEvent {
    fn event_tag() -> &'static str {
        "snake_game_over"
    }
}

/// Raised on server when the snake eats a gem; pipelines react to it as `snake.gem_eaten`
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct SnakeGemEatenEvent {
    pub session_id: Uuid,
    pub gems_eaten: u32,
}
