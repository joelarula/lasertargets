use bevy::prelude::*;

pub mod collision;
pub mod input;
pub mod lifecycle;
pub mod movement;
pub mod render;
pub mod spawn;

/// System sets for explicit execution ordering of the Hunter minigame
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum HunterSystemSet {
    Input,
    Spawn,
    Movement,
    Collision,
    Render,
    Lifecycle,
}
