use bevy::asset::uuid::Uuid;
use bevy::math::Vec3;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::audit::{HunterTally, ShotSource};
use crate::types::HunterTarget;

/// A shot at a world position (server)
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct HunterClickEvent {
    pub source: ShotSource,
    pub click_position: Vec3,
}

/// Running totals, sent to terminals whenever they change
#[derive(Message, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BroadcastStatsUpdateEvent {
    pub session_id: Uuid,
    pub tally: HunterTally,
}

/// Request to place a target at a world position (from input, the workflow or a terminal)
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct SpawnHunterTargetEvent {
    pub target: HunterTarget,
    pub position: Vec3,
}

impl common::curator_core::CuratorPayload for BroadcastStatsUpdateEvent {
    fn event_tag() -> &'static str {
        "hunter_stats"
    }
}

impl common::curator_core::CuratorPayload for SpawnHunterTargetEvent {
    fn event_tag() -> &'static str {
        "spawn_target"
    }
}
