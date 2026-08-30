use bevy::math::Mat4;
use bevy::prelude::*;
use common::path::{PathRenderable, UniversalPath};
use common::scene::{SceneEntity, SceneSetup};
use common::target::HunterTarget;

use crate::components::{CollisionIndicator, HunterShotRipple, HunterTargetEntity, TargetSpawnImmunity};
use crate::events::{BroadcastStatsUpdateEvent, HunterClickEvent};
use crate::resources::HunterGameStats;

/// Handle click events from clients and detect collisions server-side
pub fn handle_hunter_clicks(
    mut commands: Commands,
    mut click_events: MessageReader<HunterClickEvent>,
    target_query: Query<(Entity, &Transform, Option<&ChildOf>, &HunterTargetEntity, Option<&TargetSpawnImmunity>)>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    scene_setup: Res<SceneSetup>,
    mut stats: Option<ResMut<HunterGameStats>>,
    time: Res<Time>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
    indicator_query: Query<Entity, With<CollisionIndicator>>,
) {
    for event in click_events.read() {
        let click_pos = event.click_position;
        let scene_result = scene_query.single().ok();
        let scene_transform = scene_result.map(|(_, t)| t);
        let mut hit_any = false;
        
        for (entity, transform, parent, target_entity, immunity) in &target_query {
            if target_entity.session_id != event.session_id || immunity.is_some() {
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
                if let Some(stats) = stats.as_mut() {
                    stats.targets_popped += 1;
                    stats.score += target_entity.reward;
                    
                    let elapsed = time.elapsed_secs_f64() - stats.game_start_time;
                    stats.target_events.push(crate::types::TargetEvent {
                        target_uuid: target_entity.uuid,
                        event_type: "popped".to_string(),
                        timestamp: elapsed,
                        position: target_pos,
                    });
                    
                    stats_events.write(BroadcastStatsUpdateEvent {
                        session_id: event.session_id,
                        targets_spawned: stats.targets_spawned,
                        targets_popped: stats.targets_popped,
                        misses: stats.misses,
                        score: stats.score,
                    });
                    
                    info!("Target {} popped at {:?}, score: {}", target_entity.uuid, target_pos, stats.score);
                }
                
                commands.entity(entity).despawn();
                break;
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
                    if let Some(stats) = stats.as_mut() {
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

        for entity in indicator_query.iter() {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.despawn();
            }
        }

        if let Some(scene_transform) = scene_transform {
            let scene_matrix = Mat4::from_scale_rotation_translation(
                scene_transform.scale,
                scene_transform.rotation,
                scene_transform.translation,
            );
            let local_click = scene_matrix.inverse().transform_point3(click_pos);

            let dot_color = if hit_any {
                Color::srgb(1.0, 0.95, 0.1)
            } else {
                Color::srgb(1.0, 0.1, 0.0)
            };

            let indicator_path = UniversalPath::circle(
                Vec2::ZERO,
                0.05,
                dot_color,
            );

            let indicator_transform = Transform::from_translation(local_click);
            let indicator_entity = commands.spawn((
                CollisionIndicator,
                HunterShotRipple {
                    current_radius: 0.05,
                    max_radius: 0.35,
                    growth_rate: 3.0,
                    color: dot_color,
                },
                indicator_transform,
                GlobalTransform::from(indicator_transform),
                Visibility::default(),
                indicator_path,
                PathRenderable::default(),
            )).id();

            if let Some((scene_entity, _)) = scene_result {
                commands.entity(scene_entity).add_child(indicator_entity);
            }
            info!("🎯 Spawned expanding shot ripple ring at {:?}", local_click);
        }
    }
}
