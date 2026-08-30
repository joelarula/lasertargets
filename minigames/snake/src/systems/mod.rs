use bevy::prelude::*;

pub mod input;
pub mod lifecycle;
pub mod movement;
pub mod render;

/// System sets for explicit execution ordering of the Snake minigame
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum SnakeSystemSet {
    Input,
    Movement,
    Render,
    Lifecycle,
}
