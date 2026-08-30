use bevy::prelude::*;
use common::scene::SceneSetup;
use common::target::HunterTarget;

use crate::components::{BalloonRiseSpeed, BalloonTargetEntity, HunterTargetEntity};
use crate::events::BroadcastStatsUpdateEvent;
use crate::resources::HunterGameStats;

/// Move balloon targets upward each fixed tick
pub fn update_balloon_positions(
    mut balloon_query: Query<(&mut Transform, &BalloonRiseSpeed), With<BalloonTargetEntity>>,
    time: Res<Time>,
) {
    for (mut transform, speed) in balloon_query.iter_mut() {
        transform.translation.y += speed.0 * time.delta_secs();
    }
}

/// Despawn balloons that have risen past the top of the scene
pub fn check_balloon_out_of_bounds(
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
            info!("Balloon {} escaped at y={:.2}", target.uuid, transform.translation.y);
            
            if let Some(stats) = stats.as_mut() {
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
