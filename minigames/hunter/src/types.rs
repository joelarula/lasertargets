use bevy::asset::uuid::Uuid;
use bevy::math::Vec3;
use common::path::UniversalPath;
use common::scene::SceneSetup;
use serde::{Deserialize, Serialize};

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
    pub position: Vec3,
}

/// Post-game report with analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameReport {
    pub scene_setup: SceneSetup,
    pub total_targets_spawned: u32,
    pub total_targets_popped: u32,
    pub total_misses: u32,
    pub total_score: u32,
    pub total_game_time: f64,
    pub avg_spawn_interval: f64,
    pub avg_target_lifetime: f64,
    pub spawn_positions: Vec<Vec3>,
    pub pop_positions: Vec<Vec3>,
    pub timeline: Vec<TargetEvent>,
}
