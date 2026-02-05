use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use bevy_quinnet::server::QuinnetServer;
use common::network::NetworkMessage;
use common::path::{PathRenderable, UniversalPath};
use common::scene::SceneEntity;

/// Component to track the UUID of a path entity for network synchronization
#[derive(Component, Debug, Clone)]
pub struct PathId(pub Uuid);

pub struct PathNetworkPlugin;

impl Plugin for PathNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, broadcast_path_spawns)
            .add_systems(Update, broadcast_path_despawns)
            .add_systems(Update, broadcast_path_position_updates);
    }
}

/// Broadcast newly spawned path entities to all clients
fn broadcast_path_spawns(
    mut server: ResMut<QuinnetServer>,
    mut commands: Commands,
    new_paths: Query<
        (Entity, &UniversalPath, &GlobalTransform),
        (Added<UniversalPath>, With<PathRenderable>),
    >,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for (entity, path, global_transform) in new_paths.iter() {
        // Generate a UUID for this path if it doesn't have one
        let path_id = Uuid::new_v4();
        commands.entity(entity).insert(PathId(path_id));

        let position = global_transform.translation();
        
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
    path_ids: Query<&PathId>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for entity in removed.read() {
        // Try to get the PathId before it's removed
        if let Ok(path_id) = path_ids.get(entity) {
            info!("Broadcasting DespawnPath: uuid={}", path_id.0);

            let message = NetworkMessage::DespawnPath(path_id.0);
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
        (&PathId, &GlobalTransform),
        (
            Changed<GlobalTransform>,
            With<UniversalPath>,
            With<PathRenderable>,
        ),
    >,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for (path_id, global_transform) in changed_paths.iter() {
        let position = global_transform.translation();
        
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
