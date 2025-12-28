use crate::actor::Actor;
use serde::{Deserialize, Serialize};
use bevy::{
    asset::uuid::Uuid, ecs::resource::Resource
};



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub name: String,
    pub uuid: Uuid,
    pub paused: bool,
    pub start_timestamp: Option<u64>,
    pub end_timestamp: Option<u64>,
    pub actors: Vec<Uuid>,
}

impl Game {

        pub fn start(&mut self, timestamp: u64) {
            if self.start_timestamp.is_none() {
                self.start_timestamp = Some(timestamp);
            }
        }

        pub fn stop(&mut self, timestamp: u64) {
            self.end_timestamp = Some(timestamp);
        }

        pub fn pause(&mut self) {
            self.paused = true;
        }

        pub fn resume(&mut self) {
            self.paused = false;
        }

    pub fn register_actor(&mut self, actor_uuid: Uuid) {
        if !self.actors.contains(&actor_uuid) {
            self.actors.push(actor_uuid);
        }
    }

    pub fn unregister_actor(&mut self, actor_uuid: Uuid) {
        self.actors.retain(|u| u != &actor_uuid);
    }
}