use bevy::prelude::*;
use common::path::UniversalPath;
use serde::{Deserialize, Serialize};

/// Points for popping a target
pub const TARGET_REWARD: u32 = 10;

/// How fast balloons rise, scene metres per second
pub const BALLOON_RISE_SPEED: f32 = 0.3;

/// Radius of the shot ripple's base circle (its pipeline grows it), scene metres
pub const RIPPLE_RADIUS: f32 = 0.05;

pub const HIT_COLOR: Color = Color::srgb(1.0, 0.95, 0.1);
pub const MISS_COLOR: Color = Color::srgb(1.0, 0.1, 0.0);

/// A target to spawn: its shape, radius in scene metres and colour
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HunterTarget {
    /// A circle that stays where it was placed
    Basic(f32, Color),
    /// A balloon that rises until shot or out of the scene
    Balloon(f32, Color),
}

impl HunterTarget {
    pub fn radius(self) -> f32 {
        match self {
            Self::Basic(radius, _) | Self::Balloon(radius, _) => radius,
        }
    }

    pub fn is_balloon(self) -> bool {
        matches!(self, Self::Balloon(..))
    }

    /// The outline the laser draws, centred on the target's position
    pub fn path(self) -> UniversalPath {
        match self {
            Self::Basic(radius, color) => UniversalPath::circle(Vec2::ZERO, radius, color),
            Self::Balloon(radius, color) => UniversalPath::balloon(Vec2::ZERO, radius, color),
        }
    }
}
