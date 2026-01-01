use bevy::{asset::uuid::Uuid, color::Color};
use common::path::UniversalPath;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
struct Hunter {
    uuid: Uuid,
    actor: Uuid,   
    score: u32,
    hits: Vec<Uuid>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
struct Target {
    name: String,
    uuid: Uuid,
    actor: Uuid,
    lives: u8,   
    reward: u32,
    path: UniversalPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HunterGame {
    game: Uuid,
    controller: Uuid,
    hunters: Vec<Hunter>,   
    targets: Vec<Target>,  
}
