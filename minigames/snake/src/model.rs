use bevy::{asset::uuid::Uuid, color::Color};
use common::path::UniversalPath;
use serde::{Deserialize, Serialize};



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snake {
    uuid: Uuid,
    actor: Uuid,
    path: UniversalPath,
    color: Color,
    lives: u8,
    score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snak {
    uuid: Uuid,
    name: String,
    actor: Uuid,
    path: UniversalPath,
    reward: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnakeGame {
    snakes: Vec<Snake>,   
    snaks: Vec<Snak>,
}
