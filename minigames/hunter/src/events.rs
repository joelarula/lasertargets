use bevy::asset::uuid::Uuid;
use bevy::math::Vec3;
use bevy::prelude::*;
use common::target::HunterTarget;
use serde::{Deserialize, Serialize};

/// Event for click detection from client (used by server)
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct HunterClickEvent {
    pub session_id: Uuid,
    pub click_position: Vec3,
}

/// Event to broadcast stats update (raised by server, sent by network plugin)
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastStatsUpdateEvent {
    pub session_id: Uuid,
    pub targets_spawned: u32,
    pub targets_popped: u32,
    pub misses: u32,
    pub score: u32,
}

/// Event for spawning hunter targets (server-only)
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct SpawnHunterTargetEvent {
    pub target: HunterTarget,
    pub position: Vec3,
}
