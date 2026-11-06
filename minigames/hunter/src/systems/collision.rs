use bevy::prelude::*;
use common::path::{PathRenderable, UniversalPath};
use common::pipeline::plugin::{DespawnOnFinish, UsePipeline};

use crate::audit::{HunterAuditEvent, HunterGameStats, NearestTarget, Shot, ShotOutcome};
use crate::components::{HunterEntity, SpawnImmunity, Target};
use crate::events::HunterClickEvent;
use common::scene::Billboard;
use crate::types::{HIT_COLOR, MISS_COLOR, RIPPLE_RADIUS};

/// Resolves shots against the targets (in scene metres), pops what was hit, logs the shot with
/// its nearest target and leaves a ripple where it landed
pub fn resolve_shots(
    mut commands: Commands,
    mut shots: MessageReader<HunterClickEvent>,
    targets: Query<(Entity, &Transform, &Target, Has<SpawnImmunity>)>,
    billboard: Billboard,
    mut stats: ResMut<HunterGameStats>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    let mut popped = Vec::new();
    for shot in shots.read() {
        let world = shot.click_position;
        let local = billboard.to_local(world);
        let aim = local.truncate();

        // (entity, target, immune, shot − centre, centre) for every target still standing
        let measured: Vec<_> = targets
            .iter()
            .filter(|(entity, ..)| !popped.contains(entity))
            .map(|(entity, transform, target, immune)| {
                (entity, target, immune, aim - transform.translation.truncate(), transform.translation)
            })
            .collect();
        let by_distance = |a: &&(_, _, _, Vec2, _), b: &&(_, _, _, Vec2, _)| a.3.length().total_cmp(&b.3.length());
        // Logged for every shot: how far off it was from the closest target
        let nearest = measured.iter().min_by(by_distance).copied();
        let hit = measured
            .iter()
            .filter(|(_, target, immune, offset, _)| !immune && offset.length() <= target.kind.radius())
            .min_by(by_distance)
            .copied();
        let outcome = match hit {
            Some((entity, target, ..)) => {
                popped.push(entity);
                commands.entity(entity).despawn();
                ShotOutcome::Hit {
                    target: target.id,
                    reward: target.reward,
                    lifetime: stats.since_start(now) - target.spawned_at,
                }
            }
            None if billboard.contains(local) => ShotOutcome::Miss,
            None => ShotOutcome::OffScene,
        };

        if outcome != ShotOutcome::OffScene {
            spawn_ripple(&mut commands, &billboard, local, hit.is_some());
        }
        stats.record(
            now,
            HunterAuditEvent::Shot(Shot {
                source: shot.source,
                world,
                local,
                outcome,
                nearest: nearest.map(|(_, target, immune, offset, at)| NearestTarget {
                    target: target.id,
                    local: at,
                    offset,
                    radius: target.kind.radius(),
                    immune,
                }),
            }),
        );
        info!("[Hunter] {:?} shot at {local}: {outcome:?} — {}", shot.source, stats.tally);
    }
}

/// Yellow (hit) or red (miss) ring; assets/pipelines/hunter/ripple.pipeline.yaml grows it and
/// removes it
fn spawn_ripple(commands: &mut Commands, billboard: &Billboard, local: Vec3, hit: bool) {
    billboard.spawn(
        commands,
        (
            HunterEntity,
            UsePipeline::new("hunter/ripple"),
            DespawnOnFinish,
            Transform::from_translation(local.with_z(0.0)),
            Visibility::default(),
            UniversalPath::circle(Vec2::ZERO, RIPPLE_RADIUS, if hit { HIT_COLOR } else { MISS_COLOR }),
            PathRenderable::default(),
        ),
    );
}
