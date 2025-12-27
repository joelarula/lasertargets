

use bevy::{platform::collections::HashMap, prelude::*};
use bevy_quinnet::shared::ClientId;
use common::actor::Actor;
use serde::{Deserialize, Serialize};


#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ActorRegistry {
    actors: HashMap<ClientId, Vec<Actor>>,
}

impl ActorRegistry {
        /// Get a reference to the actors for a client. Returns an empty slice if none exist.
        pub fn get_actors(&self, client_id: ClientId) -> &[Actor] {
            self.actors.get(&client_id).map(|v| v.as_slice()).unwrap_or(&[])
        }
    /// Register an actor for a client. If the actor with the same uuid exists, it is replaced.
    pub fn register_actor(&mut self, client_id: ClientId, actor: Actor) {
        let entry = self.actors.entry(client_id).or_default();
        // Remove any actor with the same uuid
        entry.retain(|a| a.uuid != actor.uuid);
        entry.push(actor);
    }

    /// Unregister an actor for a client by uuid.
    pub fn unregister_actor(&mut self, client_id: ClientId, uuid: bevy::asset::uuid::Uuid) {
        if let Some(entry) = self.actors.get_mut(&client_id) {
            entry.retain(|a| a.uuid != uuid);
        }
    }
}

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ActorRegistry {
            actors: HashMap::new(),
        });
    }
}