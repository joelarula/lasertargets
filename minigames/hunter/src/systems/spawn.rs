use bevy::math::Mat4;
use bevy::prelude::*;
use common::game::GameSession;
use common::path::UniversalPath;
use common::scene::SceneEntity;
use common::target::HunterTarget;
use gamepad::ServerGamepadCursor;

use crate::common::GAME_ID;
use crate::components::{BalloonRiseSpeed, BalloonTargetEntity, HunterTargetEntity, TargetSpawnImmunity};
use crate::events::{BroadcastStatsUpdateEvent, SpawnHunterTargetEvent};
use crate::resources::HunterGameStats;
use crate::types::TargetEvent;

/// Spawn hunter target entities
pub fn spawn_hunter_targets(
    mut commands: Commands,
    mut spawn_events: MessageReader<SpawnHunterTargetEvent>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    mut stats: Option<ResMut<HunterGameStats>>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
    time: Res<Time>,
    game_sessions: Query<&GameSession>,
) {
    for event in spawn_events.read() {
        info!("Spawning hunter target at {:?}", event.position);
        
        let target_uuid = bevy::asset::uuid::Uuid::new_v4();
        let reward = 10;
        let mut session_id = bevy::asset::uuid::Uuid::nil();
        
        let (radius, color, is_balloon) = match &event.target {
            HunterTarget::Basic(size, color) => (*size, *color, false),
            HunterTarget::Baloon(size, color) => (*size, *color, true),
        };
        
        let path = if is_balloon {
            UniversalPath::balloon(Vec2::ZERO, radius, color)
        } else {
            UniversalPath::circle(Vec2::ZERO, radius, color)
        };
        
        let (local_position, spawn_world_pos) = if let Ok((_scene_entity, scene_transform)) = scene_query.single() {
            let mut snapped_world_pos = event.position;
            snapped_world_pos.z = scene_transform.translation.z;

            let scene_matrix = Mat4::from_scale_rotation_translation(
                scene_transform.scale,
                scene_transform.rotation,
                scene_transform.translation,
            );
            (scene_matrix.inverse().transform_point3(snapped_world_pos), snapped_world_pos)
        } else {
            (event.position, event.position)
        };

        if let Some(stats) = stats.as_mut() {
            stats.targets_spawned += 1;
            session_id = stats.session_id;

            let elapsed = time.elapsed_secs_f64() - stats.game_start_time;
            stats.target_events.push(TargetEvent {
                target_uuid,
                event_type: "spawned".to_string(),
                timestamp: elapsed,
                position: spawn_world_pos,
            });

            stats_events.write(BroadcastStatsUpdateEvent {
                session_id: stats.session_id,
                targets_spawned: stats.targets_spawned,
                targets_popped: stats.targets_popped,
                misses: stats.misses,
                score: stats.score,
            });
        } else if let Some(session) = game_sessions.iter().find(|session| session.game_id == GAME_ID) {
            session_id = session.session_id;
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
                timestamp: 0.0,
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
        
        let radius = match &event.target {
            HunterTarget::Basic(size, _) => *size,
            HunterTarget::Baloon(size, _) => *size,
        };

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
            TargetSpawnImmunity {
                spawn_pos: event.position,
                radius,
            },
            path,
            common::path::PathRenderable::default(),
        )).id();
        
        if is_balloon {
            commands.entity(target_entity).insert((
                BalloonTargetEntity,
                BalloonRiseSpeed::default(),
            ));
        }
        
        if let Ok((scene_entity, _)) = scene_query.single() {
            commands.entity(scene_entity).add_child(target_entity);
            info!("Spawned hunter target entity as child of scene at local position {:?}", local_position);
        } else {
            warn!("No scene entity found, spawned hunter target without parent at {:?}", event.position);
        }
    }
}

/// System that checks if reticle cursor has moved outside a newly spawned target's radius.
/// Once cursor leaves, immunity is removed and target becomes shootable!
pub fn update_target_spawn_immunity(
    mut commands: Commands,
    cursor: Option<Res<ServerGamepadCursor>>,
    immunity_query: Query<(Entity, &Transform, Option<&ChildOf>, &TargetSpawnImmunity)>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
) {
    let Some(cursor) = cursor else { return; };
    let cursor_pos = cursor.position;
    let scene_transform = scene_query.single().ok().map(|(_, t)| t);

    for (entity, transform, parent, immunity) in immunity_query.iter() {
        let target_pos = if parent.is_some() {
            if let Some(scene_transform) = scene_transform {
                scene_transform.transform_point(transform.translation)
            } else {
                transform.translation
            }
        } else {
            transform.translation
        };

        let dist = cursor_pos.distance(target_pos);
        if dist > immunity.radius {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.remove::<TargetSpawnImmunity>();
                info!("✓ Cursor moved outside spawn radius — target is now shootable");
            }
        }
    }
}
