use bevy::{app::{App, Plugin, Update}, ecs::{component::Component, message::{MessageReader, MessageWriter}, system::Commands}, prelude::*};
use common::{
    game::GameSession,
    path::UniversalPath,
    scene::{SceneEntity, SceneSetup},
    state::ServerState,
    target::HunterTarget,
};
use crate::common::GAME_ID;
use crate::model::{BalloonRiseSpeed, BalloonTargetEntity, BroadcastStatsUpdateEvent, HunterClickEvent, HunterGameStats, TargetEvent};

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
        app.add_systems(Update, (spawn_hunter_targets, handle_hunter_clicks, check_balloon_out_of_bounds));
        app.add_systems(FixedUpdate, update_balloon_positions);
        app.add_systems(OnExit(ServerState::InGame), reset_hunter_session);
        app.add_systems(Update, reset_hunter_on_new_session);
    }
}

fn reset_hunter_session(
    mut commands: Commands,
    targets: Query<Entity, With<HunterTargetEntity>>,
    stats: Option<ResMut<HunterGameStats>>,
) {
    for entity in targets.iter() {
        commands.entity(entity).despawn();
    }

    if stats.is_some() {
        commands.remove_resource::<HunterGameStats>();
    }
}

fn reset_hunter_on_new_session(
    mut commands: Commands,
    mut created_events: MessageReader<common::game::GameSessionCreated>,
    targets: Query<Entity, With<HunterTargetEntity>>,
    stats: Option<ResMut<HunterGameStats>>,
) {
    for event in created_events.read() {
        if event.game_session.game_id != GAME_ID {
            continue;
        }

        for entity in targets.iter() {
            commands.entity(entity).despawn();
        }

        if stats.is_some() {
            commands.remove_resource::<HunterGameStats>();
        }
    }
}

/// Spawn hunter target entities
fn spawn_hunter_targets(
    mut commands: Commands,
    mut spawn_events: MessageReader<SpawnHunterTargetEvent>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    mut stats: Option<ResMut<HunterGameStats>>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
    time: Res<Time>,
    game_sessions: Query<&GameSession>,
    scene_setup: Res<SceneSetup>,
) {
    for event in spawn_events.read() {
        info!("Spawning hunter target at {:?}", event.position);
        
        // Generate unique UUID for this target
        let target_uuid = bevy::asset::uuid::Uuid::new_v4();
        let reward = 10; // Base reward for all targets
        
        let mut session_id = bevy::asset::uuid::Uuid::nil();
        
        // Create UniversalPath based on target type
        let (radius, color, is_balloon) = match &event.target {
            HunterTarget::Basic(size, color) => (*size, *color, false),
            HunterTarget::Baloon(size, color) => (*size, *color, true),
        };
        
        let path = if is_balloon {
            UniversalPath::balloon(Vec2::ZERO, radius, color)
        } else {
            UniversalPath::circle(Vec2::ZERO, radius, color)
        };
        
        // Get local position relative to scene transform
        let (local_position, spawn_world_pos) = if is_balloon {
            // Balloon: random X within scene bounds, start below scene bottom
            let half_width = scene_setup.scene.scene_dimension.x as f32 / 2.0;
            let half_height = scene_setup.scene.scene_dimension.y as f32 / 2.0;
            let margin = radius;
            let x = rand::random_range((-half_width + margin)..(half_width - margin));
            let local_pos = Vec3::new(x, -half_height - radius, 0.0);
            
            let world_pos = if let Ok((_scene_entity, scene_transform)) = scene_query.single() {
                let scene_matrix = Mat4::from_scale_rotation_translation(
                    scene_transform.scale,
                    scene_transform.rotation,
                    scene_transform.translation,
                );
                scene_matrix.transform_point3(local_pos)
            } else {
                local_pos
            };
            (local_pos, world_pos)
        } else if let Ok((_scene_entity, scene_transform)) = scene_query.single() {
            let mut snapped_world_pos = event.position;
            snapped_world_pos.z = scene_transform.translation.z;

            // Convert world position to local position relative to scene
            let scene_matrix = Mat4::from_scale_rotation_translation(
                scene_transform.scale,
                scene_transform.rotation,
                scene_transform.translation,
            );
            (scene_matrix.inverse().transform_point3(snapped_world_pos), snapped_world_pos)
        } else {
            // Fallback: use world position if no scene found
            (event.position, event.position)
        };

        // Update stats for this session
        if let Some(mut stats) = stats.as_mut() {
            stats.targets_spawned += 1;
            session_id = stats.session_id;

            let elapsed = time.elapsed_secs_f64() - stats.game_start_time;
            stats.target_events.push(TargetEvent {
                target_uuid,
                event_type: "spawned".to_string(),
                timestamp: elapsed,
                position: spawn_world_pos,
            });

            // Raise event for network plugin to broadcast
            stats_events.write(BroadcastStatsUpdateEvent {
                session_id: stats.session_id,
                targets_spawned: stats.targets_spawned,
                targets_popped: stats.targets_popped,
                misses: stats.misses,
                score: stats.score,
            });
        } else if let Some(session) = game_sessions.iter().find(|session| session.game_id == GAME_ID) {
            session_id = session.session_id;
            let elapsed = 0.0;
            let mut new_stats = HunterGameStats {
                session_id: session.session_id,
                targets_spawned: 1,
                targets_popped: 0,
                misses: 0,
                score: 0,
                target_events: Vec::new(),
                game_start_time: time.elapsed_secs_f64(),
            };
            new_stats.target_events.push(TargetEvent {
                target_uuid,
                event_type: "spawned".to_string(),
                timestamp: elapsed,
                position: spawn_world_pos,
            });
            commands.insert_resource(new_stats);

            stats_events.write(BroadcastStatsUpdateEvent {
                session_id: session.session_id,
                targets_spawned: 1,
                targets_popped: 0,
                misses: 0,
                score: 0,
            });
        }
        
        let transform = Transform::from_translation(local_position);
        
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
        
        // Add balloon-specific components for rising behavior
        if is_balloon {
            commands.entity(target_entity).insert((
                BalloonTargetEntity,
                BalloonRiseSpeed::default(),
            ));
        }
        
        // Parent to scene entity if it exists
        if let Ok((scene_entity, _)) = scene_query.single() {
            commands.entity(scene_entity).add_child(target_entity);
            info!("Spawned hunter target entity as child of scene at local position {:?}", local_position);
        } else {
            warn!("No scene entity found, spawned hunter target without parent at {:?}", event.position);
        }

        info!("Spawned hunter target entity {:?}", target_entity);
    }
}

/// Handle click events from clients and detect collisions server-side
fn handle_hunter_clicks(
    mut commands: Commands,
    mut click_events: MessageReader<HunterClickEvent>,
    target_query: Query<(Entity, &Transform, Option<&ChildOf>, &HunterTargetEntity)>,
    scene_query: Query<&Transform, With<SceneEntity>>,
    scene_setup: Res<SceneSetup>,
    mut stats: Option<ResMut<HunterGameStats>>,
    time: Res<Time>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
) {
    for event in click_events.read() {
        let click_pos = event.click_position;
        let scene_transform = scene_query.single().ok();
        let mut hit_any = false;
        
        // Check all targets for collision
        for (entity, transform, parent, target_entity) in &target_query {
            // Only check targets for this session
            if target_entity.session_id != event.session_id {
                continue;
            }

            let target_pos = if parent.is_some() {
                if let Some(scene_transform) = scene_transform {
                    scene_transform.transform_point(transform.translation)
                } else {
                    transform.translation
                }
            } else if let Some(scene_transform) = scene_transform {
                scene_transform.transform_point(transform.translation)
            } else {
                transform.translation
            };
            let distance = click_pos.distance(target_pos);
            
            let radius = match &target_entity.target_type {
                HunterTarget::Basic(size, _) => *size,
                HunterTarget::Baloon(size, _) => *size,
            };
            
            if distance <= radius {
                hit_any = true;
                // Target hit! Update stats
                if let Some(mut stats) = stats.as_mut() {
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
                        misses: stats.misses,
                        score: stats.score,
                    });
                    
                    info!("Target {} popped at {:?}, score: {}", target_entity.uuid, target_pos, stats.score);
                }
                
                // Despawn target (path broadcast handles visual removal)
                commands.entity(entity).despawn();
                break; // Only pop one target per click
            }
        }

        if !hit_any {
            if let Some(scene_transform) = scene_transform {
                let scene_matrix = Mat4::from_scale_rotation_translation(
                    scene_transform.scale,
                    scene_transform.rotation,
                    scene_transform.translation,
                );
                let local_click = scene_matrix.inverse().transform_point3(click_pos);
                let half_width = scene_setup.scene.scene_dimension.x as f32 / 2.0;
                let half_height = scene_setup.scene.scene_dimension.y as f32 / 2.0;

                if local_click.x.abs() <= half_width && local_click.y.abs() <= half_height {
                    if let Some(mut stats) = stats.as_mut() {
                        stats.misses += 1;
                        stats_events.write(BroadcastStatsUpdateEvent {
                            session_id: event.session_id,
                            targets_spawned: stats.targets_spawned,
                            targets_popped: stats.targets_popped,
                            misses: stats.misses,
                            score: stats.score,
                        });
                    }
                }
            }
        }
    }
}

/// Move balloon targets upward each fixed tick
fn update_balloon_positions(
    mut balloon_query: Query<(&mut Transform, &BalloonRiseSpeed), With<BalloonTargetEntity>>,
    time: Res<Time>,
) {
    for (mut transform, speed) in balloon_query.iter_mut() {
        transform.translation.y += speed.0 * time.delta_secs();
    }
}

/// Despawn balloons that have risen past the top of the scene
fn check_balloon_out_of_bounds(
    mut commands: Commands,
    balloon_query: Query<(Entity, &Transform, &HunterTargetEntity), With<BalloonTargetEntity>>,
    scene_setup: Res<SceneSetup>,
    mut stats: Option<ResMut<HunterGameStats>>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
) {
    let half_height = scene_setup.scene.scene_dimension.y as f32 / 2.0;
    
    for (entity, transform, target) in balloon_query.iter() {
        let radius = match &target.target_type {
            HunterTarget::Baloon(size, _) => *size,
            _ => 0.2,
        };
        
        if transform.translation.y > half_height + radius {
            // Balloon escaped the scene
            info!("Balloon {} escaped at y={:.2}", target.uuid, transform.translation.y);
            
            if let Some(mut stats) = stats.as_mut() {
                stats.misses += 1;
                stats_events.write(BroadcastStatsUpdateEvent {
                    session_id: target.session_id,
                    targets_spawned: stats.targets_spawned,
                    targets_popped: stats.targets_popped,
                    misses: stats.misses,
                    score: stats.score,
                });
            }
            
            commands.entity(entity).despawn();
        }
    }
}
