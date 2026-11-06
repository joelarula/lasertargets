use bevy::prelude::*;

pub mod collision;
pub mod input;
pub mod lifecycle;
pub mod movement;
pub mod render;
pub mod spawn;

/// Order of the Hunter server systems within a frame
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum HunterSystemSet {
    /// Sessions, setup changes, pauses
    Lifecycle,
    Input,
    Spawn,
    Collision,
    /// Tally broadcast to terminals
    Output,
}
