use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use std::collections::HashMap;
use common::path::{PathRenderable, UniversalPath};
use common::scene::SceneEntity;
use common::state::ServerState;
use crate::plugins::calibration::CalibrationPath;

/// Component to track the UUID of a path entity for network synchronization
#[derive(Component, Debug, Clone)]
pub struct PathId(pub Uuid);

#[derive(Message, Debug, Clone)]
pub struct BroadcastSpawnPath {
    pub uuid: Uuid,
    pub path: UniversalPath,
    pub position: Vec3,
}

#[derive(Message, Debug, Clone)]
pub struct BroadcastDespawnPath {
    pub uuid: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct BroadcastPathPosition {
    pub uuid: Uuid,
    pub position: Vec3,
}

#[derive(Resource, Default)]
pub struct PathIdRegistry {
    pub by_entity: HashMap<Entity, Uuid>,
}

pub struct PathNetworkPlugin;

impl Plugin for PathNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PathIdRegistry>()
            .add_message::<BroadcastSpawnPath>()
            .add_message::<BroadcastDespawnPath>()
            .add_message::<BroadcastPathPosition>()
            .add_systems(PostUpdate, broadcast_path_spawns)
            .add_systems(Update, broadcast_path_despawns)
            .add_systems(PostUpdate, broadcast_path_position_updates)
            .add_systems(OnExit(ServerState::InGame), cleanup_paths_on_game_exit);
    }
}

fn cleanup_paths_on_game_exit(
    mut commands: Commands,
    mut registry: ResMut<PathIdRegistry>,
    path_query: Query<Entity, (With<UniversalPath>, Without<CalibrationPath>)>,
    mut despawn_writer: MessageWriter<BroadcastDespawnPath>,
) {
    info!("Exiting ServerState::InGame");
    for entity in path_query.iter() {
        if let Some(path_id) = registry.by_entity.remove(&entity) {
            despawn_writer.write(BroadcastDespawnPath { uuid: path_id });
        }
        commands.entity(entity).despawn();
    }
    registry.by_entity.clear();
}

/// Broadcast newly spawned path entities to all clients
fn broadcast_path_spawns(
    mut commands: Commands,
    mut registry: ResMut<PathIdRegistry>,
    new_paths: Query<
        (Entity, &UniversalPath, &Transform, Option<&ChildOf>),
        (Added<UniversalPath>, With<PathRenderable>, Without<CalibrationPath>),
    >,
    scene_query: Query<&Transform, With<SceneEntity>>,
    mut spawn_writer: MessageWriter<BroadcastSpawnPath>,
) {
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

        spawn_writer.write(BroadcastSpawnPath {
            uuid: path_id,
            path: path.clone(),
            position,
        });
    }
}

/// Broadcast despawned path entities to all clients
fn broadcast_path_despawns(
    mut removed: RemovedComponents<UniversalPath>,
    mut registry: ResMut<PathIdRegistry>,
    mut despawn_writer: MessageWriter<BroadcastDespawnPath>,
) {
    for entity in removed.read() {
        if let Some(path_id) = registry.by_entity.remove(&entity) {
            info!("Broadcasting DespawnPath: uuid={}", path_id);
            despawn_writer.write(BroadcastDespawnPath { uuid: path_id });
        }
    }
}

/// Broadcast position updates for path entities when their transform changes
fn broadcast_path_position_updates(
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
    mut update_writer: MessageWriter<BroadcastPathPosition>,
) {
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

        update_writer.write(BroadcastPathPosition {
            uuid: path_id.0,
            position,
        });
    }
}
