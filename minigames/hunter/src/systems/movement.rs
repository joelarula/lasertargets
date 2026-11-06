use bevy::prelude::*;

use crate::audit::{HunterAuditEvent, HunterGameStats};
use crate::components::{Rising, Target};
use common::scene::Billboard;

/// Balloons rise each fixed tick
pub fn rise_balloons(mut rising: Query<(&mut Transform, &Rising)>, time: Res<Time>) {
    for (mut transform, rising) in &mut rising {
        transform.translation.y += rising.0 * time.delta_secs();
    }
}

/// Balloons wholly above the scene escape: logged and removed
pub fn release_escaped(
    mut commands: Commands,
    rising: Query<(Entity, &Transform, &Target), With<Rising>>,
    billboard: Billboard,
    mut stats: ResMut<HunterGameStats>,
    time: Res<Time>,
) {
    let top = billboard.half_size().y;
    for (entity, transform, target) in &rising {
        if transform.translation.y > top + target.kind.radius() {
            stats.record(
                time.elapsed_secs_f64(),
                HunterAuditEvent::Escaped { target: target.id, local: transform.translation },
            );
            info!("Balloon {} escaped at y={:.2}", target.id, transform.translation.y);
            commands.entity(entity).despawn();
        }
    }
}
