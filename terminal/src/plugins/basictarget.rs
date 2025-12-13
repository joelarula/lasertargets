use bevy::prelude::*;
use log::info;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct BasicTargetSystemSet;

#[derive(Component)]
pub struct BasicTarget {
    pub radius: f32,
    pub segments: usize,
    pub color: Color,
}

impl Default for BasicTarget {
    fn default() -> Self {
        Self {
            radius: 0.5,
            segments: 32,
            color: Color::srgb(0.0, 0.5, 1.0),
        }
    }
}

pub struct BasicTargetPlugin;

impl Plugin for BasicTargetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            draw_basic_targets.in_set(BasicTargetSystemSet),
            handle_basic_target_click.in_set(BasicTargetSystemSet),
        ));
    }
}

fn draw_basic_targets(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &BasicTarget)>,
) {
    for (transform, target) in &query {
        let pos = transform.translation;
        let segments = target.segments;
        let radius = target.radius;
        
        // Draw circle as line loop
        for i in 0..segments {
            let angle1 = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
            let angle2 = ((i + 1) as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
            
            let p1 = pos + Vec3::new(
                angle1.cos() * radius,
                angle1.sin() * radius,
                0.0
            );
            let p2 = pos + Vec3::new(
                angle2.cos() * radius,
                angle2.sin() * radius,
                0.0
            );
            
            gizmos.line(p1, p2, target.color);
        }
    }
}

fn handle_basic_target_click(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window_query: Query<&Window>,
    target_query: Query<(Entity, &Transform, &BasicTarget)>,
) {
    // Only check on mouse click
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let Ok(window) = window_query.single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Get ray from camera through cursor
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Check each target circle
    for (entity, transform, target) in &target_query {
        let circle_pos = transform.translation;
        
        // Project cursor ray onto the billboard plane (Z = circle_pos.z)
        let t = (circle_pos.z - ray.origin.z) / ray.direction.z;
        let intersection = ray.origin + ray.direction * t;
        
        // Check if click is within circle radius
        let distance_to_center = intersection.distance(circle_pos);
        
        if distance_to_center < target.radius {
            info!("Clicked on basic target at {:?}, despawning", circle_pos);
            commands.entity(entity).despawn();
        }
    }
}
