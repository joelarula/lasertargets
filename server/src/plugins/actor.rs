

use bevy::{prelude::*};
use bevy_quinnet::shared::ClientId;
use common::actor::{Actor };
use common::game::GameSession;
use bevy::asset::uuid::Uuid;
use serde::{Deserialize, Serialize};


#[derive(Component,Debug, Clone, Serialize, Deserialize)]
pub struct ActorLink{
    pub client_id: ClientId,
    pub actor: Actor,
}

// Events for actor management
#[derive(Message)]
pub struct RegisterActorEvent {
    pub client_id: ClientId,
    pub game_id: Uuid,
    pub actor: Actor,
}

#[derive(Message)]
pub struct UnregisterActorEvent {
    pub client_id: ClientId,
    pub actor_uuid: Uuid,
    pub game_uuid: Uuid,
}


#[derive(Message)]
pub struct GameActorUpdateEvent {
    pub game_uuid: Uuid,
    pub actors: Vec<ActorLink>,
}

#[derive(Message)]
pub struct ActorRegistrationResultEvent {
    pub client_id: ClientId,
    pub game_uuid: Uuid,
    pub result: Result<Actor, String>,
}

#[derive(Message)]
pub struct ActorUnregistrationResultEvent {
    pub client_id: ClientId,
    pub game_uuid: Uuid,
    pub result: Result<Actor, String>,
}



pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<RegisterActorEvent>();
        app.add_message::<UnregisterActorEvent>();
        app.add_message::<GameActorUpdateEvent>();
        app.add_message::<ActorRegistrationResultEvent>();
        app.add_message::<ActorUnregistrationResultEvent>();
        app.add_systems(Update, handle_register_actor_event);
        app.add_systems(Update, handle_unregister_actor_event);
    }
}

fn handle_register_actor_event(
    mut commands: Commands,
    mut register_actor_events: MessageReader<RegisterActorEvent>,
    game_sessions: Query<(Entity, &GameSession, Option<&Children>)>,
    actor_links: Query<&ActorLink>,
    mut actor_updates: MessageWriter<GameActorUpdateEvent>,
    mut actor_reg_results: MessageWriter<ActorRegistrationResultEvent>,
) {
    for event in register_actor_events.read() {
        
        if let Some((game_entity, _, maybe_children)) = game_sessions.iter().find(|(_, gs, _)| gs.uuid == event.game_id) {
            let actor_link = ActorLink {
                client_id: event.client_id,
                actor: event.actor.clone(),
            };

            let actor_entity = commands.spawn((event.actor.clone(), actor_link.clone())).id();
            commands.entity(game_entity).add_child(actor_entity);

            let mut actors = Vec::new();
            actors.push(actor_link);
            if let Some(children) = maybe_children {
                actors.extend(children.iter().filter_map(|child| actor_links.get(child).ok().cloned()));
            }

            actor_updates.write(GameActorUpdateEvent {
                game_uuid: event.game_id,
                actors,
            });

            actor_reg_results.write(ActorRegistrationResultEvent {
                client_id: event.client_id,
                game_uuid: event.game_id,
                result: Ok(event.actor.clone()),
            });
        }  else {
            actor_reg_results.write(ActorRegistrationResultEvent {
                client_id: event.client_id,
                game_uuid: event.game_id,
                result: Err("Game session not found".to_string()),
            });
        }
    }
}

fn handle_unregister_actor_event(
    mut commands: Commands,
    mut unregister_actor_events: MessageReader<UnregisterActorEvent>,
    game_sessions: Query<(Entity, &GameSession, Option<&Children>)>,
    actor_query: Query<(Entity, &Actor, &ActorLink)>,
    mut actor_updates: MessageWriter<GameActorUpdateEvent>,
    mut actor_unreg_results: MessageWriter<ActorUnregistrationResultEvent>,
) {
    for event in unregister_actor_events.read() {

        if let Some((game_entity, _, maybe_children)) = game_sessions.iter().find(|(_, gs, _)| gs.uuid == event.game_uuid) {

            let mut remaining: Vec<ActorLink> = Vec::new();
            let mut removed_actor: Option<Actor> = None;

            if let Some(children) = maybe_children {
                for child in children.iter() {
                    if let Ok((entity, actor_component, actor_link)) = actor_query.get(child) {
                        if actor_link.client_id == event.client_id && actor_component.uuid == event.actor_uuid {
                            commands.entity(entity).despawn();
                            removed_actor = Some(actor_component.clone());
                            continue;
                        }
                        remaining.push(actor_link.clone());
                    }
                }
            }

            actor_updates.write(GameActorUpdateEvent {
                game_uuid: event.game_uuid,
                actors: remaining,
            });

            if let Some(actor) = removed_actor {
                actor_unreg_results.write(ActorUnregistrationResultEvent {
                    client_id: event.client_id,
                    game_uuid: event.game_uuid,
                    result: Ok(actor),
                });
            } else {
                actor_unreg_results.write(ActorUnregistrationResultEvent {
                    client_id: event.client_id,
                    game_uuid: event.game_uuid,
                    result: Err("Actor not found in game session".to_string()),
                });
            }
        } else {
            actor_unreg_results.write(ActorUnregistrationResultEvent {
                client_id: event.client_id,
                game_uuid: event.game_uuid,
                result: Err("Game session not found".to_string()),
            });
        }
    }
}
