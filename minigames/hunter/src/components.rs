use bevy::asset::uuid::Uuid;
use bevy::prelude::*;

use crate::types::HunterTarget;

/// Everything the game spawns (targets, shot ripples, the title); despawned when the round ends
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HunterEntity;

/// A shootable target, positioned in scene metres under the scene entity
#[derive(Component, Debug, Clone)]
pub struct Target {
    pub id: Uuid,
    pub kind: HunterTarget,
    pub reward: u32,
    /// Audit-log time it appeared (seconds since the session started)
    pub spawned_at: f64,
}

/// Makes a target rise, scene metres per second (balloons)
#[derive(Component, Debug, Clone, Copy)]
pub struct Rising(pub f32);

/// A target placed under the reticle can't be shot until the reticle has left it
#[derive(Component, Debug, Clone, Copy)]
pub struct SpawnImmunity;
