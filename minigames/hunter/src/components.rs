use bevy::asset::uuid::Uuid;
use bevy::math::Vec3;
use bevy::prelude::*;
use common::target::HunterTarget;

/// Marker component for balloon target entities (rising targets)
#[derive(Component, Debug, Clone)]
pub struct BalloonTargetEntity;

/// Component storing the vertical rise speed for balloon targets (units/sec)
#[derive(Component, Debug, Clone)]
pub struct BalloonRiseSpeed(pub f32);

impl Default for BalloonRiseSpeed {
    fn default() -> Self {
        Self(0.3)
    }
}

/// Marker component for collision indicator
#[derive(Component, Debug, Clone)]
pub struct CollisionIndicator;

/// Component for hunter target entities
#[derive(Component, Debug, Clone)]
pub struct HunterTargetEntity {
    pub target_type: HunterTarget,
    pub uuid: Uuid,
    pub reward: u32,
    pub session_id: Uuid,
}

/// Component for title announcement text overlay
#[derive(Component, Debug, Clone)]
pub struct HunterTitleAnnouncement {
    pub timer: Timer,
}

/// Component tracking temporary spawn immunity until reticle moves away
#[derive(Component, Debug, Clone)]
pub struct TargetSpawnImmunity {
    pub spawn_pos: Vec3,
    pub radius: f32,
}

/// Component tracking an expanding shot ripple animation for Hunter game clicks
#[derive(Component, Debug, Clone)]
pub struct HunterShotRipple {
    pub current_radius: f32,
    pub max_radius: f32,
    pub growth_rate: f32,
    pub color: Color,
}
