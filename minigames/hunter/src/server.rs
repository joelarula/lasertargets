use bevy::{app::{App, Plugin, Update}, ecs::{component::Component, message::MessageReader, system::Commands}, prelude::*};
use common::{path::UniversalPath, scene::SceneEntity, target::HunterTarget};

/// Event for spawning hunter targets
#[derive(Message, Debug, Clone)]
pub struct SpawnHunterTargetEvent {
    pub target: HunterTarget,
    pub position: Vec3,
}

/// Component for hunter target entities
#[derive(Component)]
pub struct HunterTargetEntity {
    pub target_type: HunterTarget,
}

pub struct HunterGameServerPlugin;

impl Plugin for HunterGameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnHunterTargetEvent>();
        app.add_systems(Update, spawn_hunter_targets);
    }
}

/// Spawn hunter target entities
fn spawn_hunter_targets(
    mut commands: Commands,
    mut spawn_events: MessageReader<SpawnHunterTargetEvent>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
) {
    for event in spawn_events.read() {
        info!("Spawning hunter target at {:?}", event.position);
        
        // Create UniversalPath based on target type
        let (radius, color) = match &event.target {
            HunterTarget::Basic(size, color) => (*size, *color),
            HunterTarget::Baloon(size, color) => (*size, *color),
        };
        
        let path = UniversalPath::circle(Vec2::new(event.position.x, event.position.y), radius, color);
        
        // Get local position relative to scene transform
        let local_position = if let Ok((scene_entity, scene_transform)) = scene_query.single() {
            // Convert world position to local position relative to scene
            let scene_matrix = Mat4::from_scale_rotation_translation(
                scene_transform.scale,
                scene_transform.rotation,
                scene_transform.translation,
            );
            scene_matrix.inverse().transform_point3(event.position)
        } else {
            // Fallback: use world position if no scene found
            event.position
        };
        
        let transform = Transform::from_translation(local_position);
        
        let target_entity = commands.spawn((
            transform,
            GlobalTransform::from(transform),
            Visibility::default(),
            HunterTargetEntity {
                target_type: event.target.clone(),
            },
            path,
            common::path::PathRenderable::default(),
        )).id();
        
        // Parent to scene entity if it exists
        if let Ok((scene_entity, _)) = scene_query.single() {
            commands.entity(scene_entity).add_child(target_entity);
            info!("Spawned hunter target entity as child of scene at local position {:?}", local_position);
        } else {
            warn!("No scene entity found, spawned hunter target without parent at {:?}", event.position);
        }
    }
}

