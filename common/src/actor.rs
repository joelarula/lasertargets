use serde::{Deserialize, Serialize};
use bevy::{
    asset::uuid::Uuid, ecs::resource::Resource
};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ActorMetaData {
    pub actors: Vec<Actor>,
}


#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub name: String,
    pub uuid: Uuid,
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Server,
    Controller,
    Player,
    Spectator,
}
