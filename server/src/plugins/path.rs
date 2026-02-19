use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use std::collections::HashMap;
use bevy_quinnet::server::QuinnetServer;
use common::network::NetworkMessage;
use common::path::{PathRenderable, UniversalPath};
use common::scene::SceneEntity;
use crate::plugins::calibration::CalibrationPath;

/// Component to track the UUID of a path entity for network synchronization
#[derive(Component, Debug, Clone)]
pub struct PathId(pub Uuid);

#[derive(Resource, Default)]
pub struct PathIdRegistry {
    pub by_entity: HashMap<Entity, Uuid>,
}

pub struct PathNetworkPlugin;

impl Plugin for PathNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PathIdRegistry>()
            .add_systems(PostUpdate, broadcast_path_spawns)
            .add_systems(Update, broadcast_path_despawns)
            .add_systems(PostUpdate, broadcast_path_position_updates);
    }
}

/// Broadcast newly spawned path entities to all clients
fn broadcast_path_spawns(
    mut server: ResMut<QuinnetServer>,
    mut commands: Commands,
    mut registry: ResMut<PathIdRegistry>,
    new_paths: Query<
        (Entity, &UniversalPath, &Transform, Option<&ChildOf>),
        (Added<UniversalPath>, With<PathRenderable>, Without<CalibrationPath>),
    >,
    scene_query: Query<&Transform, With<SceneEntity>>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for (entity, path, transform, parent) in new_paths.iter() {
        // Generate a UUID for this path if it doesn't have one
        let path_id = Uuid::new_v4();
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.queue_silenced(move |mut entity: bevy::ecs::world::EntityWorldMut<'_>| {
                entity.insert(PathId(path_id));
            });
        } else {
            warn!("Skipped PathId insert; entity {:?} no longer exists", entity);
            continue;
        }

        registry.by_entity.insert(entity, path_id);

        let position = if let Some(parent) = parent {
            if let Ok(scene_transform) = scene_query.get(parent.parent()) {
                scene_transform.transform_point(transform.translation)
            } else {
                transform.translation
            }
        } else if let Ok(scene_transform) = scene_query.single() {
            scene_transform.transform_point(transform.translation)
        } else {
            transform.translation
        };
        
        info!(
            "Broadcasting SpawnPath: uuid={}, position={:?}",
            path_id, position
        );

        let message = NetworkMessage::SpawnPath(path_id, path.clone(), position);
        if let Ok(payload) = message.to_bytes() {
            if let Err(e) = endpoint.broadcast_payload(payload) {
                error!("Failed to broadcast SpawnPath: {}", e);
            }
        }
    }
}

/// Broadcast despawned path entities to all clients
fn broadcast_path_despawns(
    mut server: ResMut<QuinnetServer>,
    mut removed: RemovedComponents<UniversalPath>,
    mut registry: ResMut<PathIdRegistry>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for entity in removed.read() {
        if let Some(path_id) = registry.by_entity.remove(&entity) {
            info!("Broadcasting DespawnPath: uuid={}", path_id);

            let message = NetworkMessage::DespawnPath(path_id);
            if let Ok(payload) = message.to_bytes() {
                if let Err(e) = endpoint.broadcast_payload(payload) {
                    error!("Failed to broadcast DespawnPath: {}", e);
                }
            }
        }
    }
}

/// Broadcast position updates for path entities when their transform changes
fn broadcast_path_position_updates(
    mut server: ResMut<QuinnetServer>,
    changed_paths: Query<
        (&PathId, &Transform, Option<&ChildOf>),
        (
            Changed<Transform>,
            With<UniversalPath>,
            With<PathRenderable>,
            Without<CalibrationPath>,
        ),
    >,
    scene_query: Query<&Transform, With<SceneEntity>>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for (path_id, transform, parent) in changed_paths.iter() {
        let position = if let Some(parent) = parent {
            if let Ok(scene_transform) = scene_query.get(parent.parent()) {
                scene_transform.transform_point(transform.translation)
            } else {
                transform.translation
            }
        } else if let Ok(scene_transform) = scene_query.single() {
            scene_transform.transform_point(transform.translation)
        } else {
            transform.translation
        };
        
        debug!(
            "Broadcasting UpdatePathPosition: uuid={}, position={:?}",
            path_id.0, position
        );

        let message = NetworkMessage::UpdatePathPosition(path_id.0, position);
        if let Ok(payload) = message.to_bytes() {
            if let Err(e) = endpoint.broadcast_payload(payload) {
                error!("Failed to broadcast UpdatePathPosition: {}", e);
            }
        }
    }
}
