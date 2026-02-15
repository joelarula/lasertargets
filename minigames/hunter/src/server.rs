use bevy::{app::{App, Plugin, Update}, ecs::{component::Component, message::{MessageReader, MessageWriter}, system::Commands}, prelude::*};
use common::{
    path::UniversalPath,
    scene::SceneEntity,
    target::HunterTarget,
};
use crate::model::{BroadcastStatsUpdateEvent, HunterClickEvent, HunterGameStats};

/// Event for spawning hunter targets (server-only)
#[derive(Message, Debug, Clone)]
pub struct SpawnHunterTargetEvent {
    pub target: HunterTarget,
    pub position: Vec3,
}

/// Component for hunter target entities
#[derive(Component)]
pub struct HunterTargetEntity {
    pub target_type: HunterTarget,
    pub uuid: bevy::asset::uuid::Uuid,
    pub reward: u32,
    pub session_id: bevy::asset::uuid::Uuid,
}

pub struct HunterGameServerPlugin;

impl Plugin for HunterGameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnHunterTargetEvent>();
        app.add_message::<HunterClickEvent>();
        app.add_message::<BroadcastStatsUpdateEvent>();
        app.add_systems(Update, (spawn_hunter_targets, handle_hunter_clicks));
    }
}

/// Spawn hunter target entities
fn spawn_hunter_targets(
    mut commands: Commands,
    mut spawn_events: MessageReader<SpawnHunterTargetEvent>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    stats_query: Query<&HunterGameStats>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
) {
    for event in spawn_events.read() {
        info!("Spawning hunter target at {:?}", event.position);
        
        // Generate unique UUID for this target
        let target_uuid = bevy::asset::uuid::Uuid::new_v4();
        let reward = 10; // Base reward for all targets
        
        // Update stats for this session
        if let Ok(stats) = stats_query.single() {
            let updated_spawned = stats.targets_spawned + 1;
            
            // Raise event for network plugin to broadcast
            stats_events.write(BroadcastStatsUpdateEvent {
                session_id: stats.session_id,
                targets_spawned: updated_spawned,
                targets_popped: stats.targets_popped,
                score: stats.score,
            });
        }
        
        // Create UniversalPath based on target type
        let (radius, color) = match &event.target {
            HunterTarget::Basic(size, color) => (*size, *color),
            HunterTarget::Baloon(size, color) => (*size, *color),
        };
        
        let path = UniversalPath::circle(Vec2::new(event.position.x, event.position.y), radius, color);
        
        // Get local position relative to scene transform
        let local_position = if let Ok((_scene_entity, scene_transform)) = scene_query.single() {
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
        
        // Get session_id from stats if available
        let session_id = if let Ok(stats) = stats_query.single() {
            stats.session_id
        } else {
            bevy::asset::uuid::Uuid::nil()
        };
        
        let target_entity = commands.spawn((
            transform,
            GlobalTransform::from(transform),
            Visibility::default(),
            HunterTargetEntity {
                target_type: event.target.clone(),
                uuid: target_uuid,
                reward,
                session_id,
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

/// Handle click events from clients and detect collisions server-side
fn handle_hunter_clicks(
    mut commands: Commands,
    mut click_events: MessageReader<HunterClickEvent>,
    target_query: Query<(Entity, &GlobalTransform, &HunterTargetEntity)>,
    mut stats_query: Query<&mut HunterGameStats>,
    time: Res<Time>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
) {
    for event in click_events.read() {
        let click_pos = event.click_position;
        
        // Check all targets for collision
        for (entity, global_transform, target_entity) in &target_query {
            // Only check targets for this session
            if target_entity.session_id != event.session_id {
                continue;
            }
            
            let target_pos = global_transform.translation();
            let distance = click_pos.distance(target_pos);
            
            // Targets are 0.25m diameter (0.125m radius)
            let radius = 0.125;
            
            if distance <= radius {
                // Target hit! Update stats
                if let Ok(mut stats) = stats_query.single_mut() {
                    stats.targets_popped += 1;
                    stats.score += target_entity.reward;
                    
                    // Track event
                    let elapsed = time.elapsed_secs_f64() - stats.game_start_time;
                    stats.target_events.push(crate::model::TargetEvent {
                        target_uuid: target_entity.uuid,
                        event_type: "popped".to_string(),
                        timestamp: elapsed,
                        position: target_pos,
                    });
                    
                    // Broadcast stats update (path despawn is automatic)
                    stats_events.write(BroadcastStatsUpdateEvent {
                        session_id: event.session_id,
                        targets_spawned: stats.targets_spawned,
                        targets_popped: stats.targets_popped,
                        score: stats.score,
                    });
                    
                    info!("Target {} popped at {:?}, score: {}", target_entity.uuid, target_pos, stats.score);
                }
                
                // Despawn target (path broadcast handles visual removal)
                commands.entity(entity).despawn();
                break; // Only pop one target per click
            }
        }
    }
}

