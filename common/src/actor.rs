use serde::{Deserialize, Serialize};
use bevy::{asset::uuid::Uuid, prelude::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorMetaData {
    pub actors: Vec<Actor>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub name: String,
    pub uuid: Uuid,
    pub roles: Vec<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Server,
    Controller,
    Player,
    Spectator,
}

