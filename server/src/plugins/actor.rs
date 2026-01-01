

use bevy::{prelude::*};
use bevy_quinnet::shared::ClientId;
use common::actor::{Actor };
use bevy::asset::uuid::Uuid;
use serde::{Deserialize, Serialize};


#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorClientId(pub ClientId);


// Events for actor management
#[derive(Message)]
pub struct RegisterActorEvent {
    pub client_id: ClientId,
    pub actor: Actor,
}

#[derive(Message)]
pub struct UnregisterActorEvent {
    pub client_id: ClientId,
    pub actor_uuid: Uuid,
}


pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RegisterActorEvent>();
        app.add_message::<UnregisterActorEvent>();
        app.add_systems(Update, handle_register_actor_event);
        app.add_systems(Update, handle_unregister_actor_event);
    }
}

fn handle_register_actor_event(
    mut commands: Commands,
    mut register_actor_events: MessageReader<RegisterActorEvent>,
) {
    for event in register_actor_events.read() {
        info!("Registering actor: {:?}", event.actor);
        commands.spawn((event.actor.clone(), ActorClientId(event.client_id)));
    }
}

fn handle_unregister_actor_event(
    mut commands: Commands,
    mut unregister_actor_events: MessageReader<UnregisterActorEvent>,
    actor_query: Query<(Entity, &Actor, &ActorClientId)>,
) {
    for event in unregister_actor_events.read() {
        info!("Unregistering actor with UUID: {:?}", event.actor_uuid);
        for (entity, actor_component, actor_client_id) in actor_query.iter() {
            if actor_client_id.0 == event.client_id && actor_component.uuid == event.actor_uuid {
                commands.entity(entity).despawn();
                return;
            }
        }
    }
}

