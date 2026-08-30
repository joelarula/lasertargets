use bevy::prelude::*;

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

/// Marker: title announcement text entity with auto-despawn timer
#[derive(Component, Debug, Clone)]
pub struct SnakeTitleAnnouncement {
    pub timer: Timer,
}
