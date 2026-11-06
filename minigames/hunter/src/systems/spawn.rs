use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::path::PathRenderable;
use gamepad::ServerGamepadCursor;

use crate::audit::{HunterAuditEvent, HunterGameStats};
use crate::components::{HunterEntity, Rising, SpawnImmunity, Target};
use crate::events::SpawnHunterTargetEvent;
use common::scene::Billboard;
use crate::types::{BALLOON_RISE_SPEED, TARGET_REWARD};

/// Places requested targets on the scene plane and logs them
pub fn spawn_targets(
    mut commands: Commands,
    mut requests: MessageReader<SpawnHunterTargetEvent>,
    billboard: Billboard,
    mut stats: ResMut<HunterGameStats>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    for request in requests.read() {
        let id = Uuid::new_v4();
        let kind = request.target;
        // Onto the scene plane, wherever the request came from
        let local = billboard.to_local(request.position).with_z(0.0);
        let world = billboard.to_world(local);

        let entity = billboard.spawn(
            &mut commands,
            (
                HunterEntity,
                Target { id, kind, reward: TARGET_REWARD, spawned_at: stats.since_start(now) },
                SpawnImmunity,
                Transform::from_translation(local),
                Visibility::default(),
                kind.path(),
                PathRenderable::default(),
            ),
        );
        if kind.is_balloon() {
            commands.entity(entity).insert(Rising(BALLOON_RISE_SPEED));
        }
        stats.record(now, HunterAuditEvent::Spawned { target: id, kind, world, local });
        info!("Spawned {kind:?} {id} at {local} (world {world})");
    }
}

/// A target spawned under the reticle becomes shootable once the reticle has left it
pub fn lift_spawn_immunity(
    mut commands: Commands,
    cursor: Option<Res<ServerGamepadCursor>>,
    billboard: Billboard,
    immune: Query<(Entity, &Transform, &Target), With<SpawnImmunity>>,
) {
    // Without a reticle there is nothing to move away
    let cursor = cursor.map(|c| billboard.to_local(c.position));
    for (entity, transform, target) in &immune {
        if cursor.is_none_or(|c| c.truncate().distance(transform.translation.truncate()) > target.kind.radius()) {
            commands.entity(entity).remove::<SpawnImmunity>();
        }
    }
}
