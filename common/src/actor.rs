use serde::{Deserialize, Serialize};
use bevy::{
    asset::uuid::Uuid, ecs::resource::Resource
};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ActorMetaData {
    actors: Vec<Actor>,
}


#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    name: String,
    uuid: Uuid,
    roles: Vec<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Server,
    Controller,
    Player,
    Spectator,
}
