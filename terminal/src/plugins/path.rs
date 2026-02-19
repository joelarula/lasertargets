use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::path::{PathRenderable, UniversalPath};
use common::scene::SceneEntity;
use std::collections::HashMap;

/// Component to track the UUID of a path entity for network synchronization
#[derive(Component, Debug, Clone)]
pub struct PathId(pub Uuid);

/// Resource to track spawned path entities by their UUID
#[derive(Resource, Default)]
pub struct PathRegistry {
    pub paths: HashMap<Uuid, Entity>,
}

/// Extension trait to add gizmo drawing to UniversalPath
trait UniversalPathGizmos {
    fn draw_with_gizmos(&self, gizmos: &mut Gizmos, transform: &GlobalTransform);
}

impl UniversalPathGizmos for UniversalPath {
    fn draw_with_gizmos(&self, gizmos: &mut Gizmos, transform: &GlobalTransform) {
        for segment in &self.segments {
            if segment.points.len() < 2 {
                continue;
            }
            
            // Draw lines between consecutive points
            for i in 0..segment.points.len() - 1 {
                let start_point = &segment.points[i];
                let end_point = &segment.points[i + 1];
                
                let start = transform.transform_point(Vec3::new(start_point.x, start_point.y, 0.0));
                let end = transform.transform_point(Vec3::new(end_point.x, end_point.y, 0.0));
                
                let color = Color::srgb(
                    start_point.r as f32 / 255.0,
                    start_point.g as f32 / 255.0,
                    start_point.b as f32 / 255.0,
                );
                
                gizmos.line(start, end, color);
            }
        }
    }
}

/// Events for path operations
#[derive(Message, Debug, Clone)]
pub struct SpawnPathEvent {
    pub uuid: Uuid,
    pub path: UniversalPath,
    pub position: Vec3,
}

#[derive(Message, Debug, Clone)]
pub struct DespawnPathEvent {
    pub uuid: Uuid,
}

#[derive(Message, Debug, Clone)]
pub struct UpdatePathPositionEvent {
    pub uuid: Uuid,
    pub position: Vec3,
}

pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PathRegistry>()
            .add_message::<SpawnPathEvent>()
            .add_message::<DespawnPathEvent>()
            .add_message::<UpdatePathPositionEvent>()
            .add_systems(Update, handle_spawn_path_events)
            .add_systems(Update, handle_despawn_path_events)
            .add_systems(Update, handle_update_path_position_events)
            .add_systems(Update, attach_paths_to_scene)
            .add_systems(Update, draw_paths);
    }
}

/// Handle spawn path events and create entities under the scene
fn handle_spawn_path_events(
    mut commands: Commands,
    mut spawn_events: MessageReader<SpawnPathEvent>,
    mut path_registry: ResMut<PathRegistry>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
) {
    for event in spawn_events.read() {
        info!(
            "Spawning path entity: uuid={}, position={:?}",
            event.uuid, event.position
        );

        let mut transform = Transform::from_translation(event.position);

        let path_entity = commands
            .spawn((
                transform,
                GlobalTransform::from(transform),
                Visibility::default(),
                PathId(event.uuid),
                event.path.clone(),
                PathRenderable::default(),
            ))
            .id();

        // Parent to scene entity if it exists
        if let Ok((scene_entity, scene_transform)) = scene_query.single() {
            let local_pos = scene_transform
                .to_matrix()
                .inverse()
                .transform_point3(event.position);
            transform.translation = local_pos;
            commands.entity(path_entity).insert(transform);
            commands.entity(scene_entity).add_child(path_entity);
            info!("Spawned path entity as child of scene");
        } else {
            warn!("No scene entity found, spawned path without parent");
        }

        // Track the entity in the registry
        path_registry.paths.insert(event.uuid, path_entity);
    }
}

/// Attach any unparented paths to the scene and convert to local coordinates
fn attach_paths_to_scene(
    mut commands: Commands,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    path_query: Query<(Entity, &GlobalTransform), With<PathId>>,
) {
    let Ok((scene_entity, scene_transform)) = scene_query.single() else {
        return;
    };

    let scene_inverse = scene_transform.to_matrix().inverse();

    for (entity, global_transform) in path_query.iter() {
        let world_pos: Vec3 = global_transform.translation();
        let local_pos = scene_inverse.transform_point3(world_pos);
        if let Ok(mut entity_commands) = commands.get_entity(entity) {
            entity_commands.queue_silenced(move |mut entity: bevy::ecs::world::EntityWorldMut<'_>| {
                entity.insert(Transform::from_translation(local_pos));
                entity.insert(ChildOf(scene_entity));
            });
        }
    }
}

/// Handle despawn path events and remove entities
fn handle_despawn_path_events(
    mut commands: Commands,
    mut despawn_events: MessageReader<DespawnPathEvent>,
    mut path_registry: ResMut<PathRegistry>,
) {
    for event in despawn_events.read() {
        info!("Despawning path entity: uuid={}", event.uuid);

        if let Some(entity) = path_registry.paths.remove(&event.uuid) {
            commands.entity(entity).despawn();
            info!("Despawned path entity");
        } else {
            warn!("Path entity not found in registry: uuid={}", event.uuid);
        }
    }
}

/// Handle update path position events
fn handle_update_path_position_events(
    mut update_events: MessageReader<UpdatePathPositionEvent>,
    path_registry: Res<PathRegistry>,
    mut transform_queries: ParamSet<(
        Query<&mut Transform, With<PathId>>,
        Query<&Transform, With<SceneEntity>>,
    )>,
) {
    let scene_inverse = if let Ok(scene_transform) = transform_queries.p1().single() {
        Some(scene_transform.to_matrix().inverse())
    } else {
        None
    };

    for event in update_events.read() {
        if let Some(&entity) = path_registry.paths.get(&event.uuid) {
            if let Ok(mut transform) = transform_queries.p0().get_mut(entity) {
                if let Some(scene_inverse) = scene_inverse {
                    transform.translation = scene_inverse.transform_point3(event.position);
                } else {
                    transform.translation = event.position;
                }
                debug!(
                    "Updated path position: uuid={}, position={:?}",
                    event.uuid, event.position
                );
            }
        } else {
            warn!(
                "Path entity not found for position update: uuid={}",
                event.uuid
            );
        }
    }
}

/// Draw all path entities as gizmos
fn draw_paths(
    mut gizmos: Gizmos,
    path_query: Query<(&GlobalTransform, &UniversalPath), With<PathRenderable>>,
) {
    for (global_transform, path) in &path_query {
        path.draw_with_gizmos(&mut gizmos, global_transform);
    }
}
