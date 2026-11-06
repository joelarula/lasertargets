use bevy::prelude::*;

pub mod input;
pub mod lifecycle;
pub mod movement;
pub mod render;

/// Server-side order of the Snake minigame within a frame (chained, before the pipelines run)
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SnakeSystemSet {
    /// Start and stop games as sessions come and go
    Lifecycle,
    /// Keys and gamepad become direction changes
    Input,
    /// Move the snake; end and restart rounds
    Movement,
    /// Write base shapes, signals and stats from the state
    Output,
}
