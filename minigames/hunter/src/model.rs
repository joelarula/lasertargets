use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::path::UniversalPath;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hunter {
    pub uuid: Uuid,
    pub actor: Uuid,   
    pub score: u32,
    pub hits: Vec<Uuid>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub name: String,
    pub uuid: Uuid,
    pub actor: Uuid,
    pub lives: u8,   
    pub reward: u32,
    pub path: UniversalPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HunterGame {
    pub game: Uuid,
    pub controller: Uuid,
    pub hunters: Vec<Hunter>,   
    pub targets: Vec<Target>,  
}

/// Event tracking for targets (spawned or popped)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetEvent {
    pub target_uuid: Uuid,
    pub event_type: String, // "spawned" or "popped"
    pub timestamp: f64,
    pub position: bevy::math::Vec3,
}

/// Marker component for collision indicator
#[derive(bevy::ecs::component::Component, Debug, Clone)]
pub struct CollisionIndicator;

/// Resource for tracking game statistics
#[derive(bevy::ecs::component::Component, bevy::prelude::Resource, Debug, Clone, Serialize, Deserialize, Default)]
pub struct HunterGameStats {
    pub session_id: Uuid,
    pub targets_spawned: u32,
    pub targets_popped: u32,
    pub misses: u32,
    pub score: u32,
    pub target_events: Vec<TargetEvent>,
    pub game_start_time: f64,
}

/// Post-game report with analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameReport {
    pub total_targets_spawned: u32,
    pub total_targets_popped: u32,
    pub total_score: u32,
    pub total_game_time: f64,
    pub avg_spawn_interval: f64,
    pub avg_target_lifetime: f64,
    pub spawn_positions: Vec<bevy::math::Vec3>,
    pub pop_positions: Vec<bevy::math::Vec3>,
    pub timeline: Vec<TargetEvent>,
}
