use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use log::info;
use common::path::{UniversalPath, PathProvider, PathRenderable};

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

impl PathProvider for BasicTarget {
    fn to_universal_path(&self) -> UniversalPath {
        UniversalPath::circle(Vec2::ZERO, self.radius, self.color)
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
    query: Query<(&GlobalTransform, &BasicTarget), With<PathRenderable>>,
) {
    for (global_transform, target) in &query {
        let path = target.to_universal_path();
        path.draw_with_gizmos(&mut gizmos, global_transform, 0.1);
    }
}

fn handle_basic_target_click(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window_query: Query<&Window>,
    target_query: Query<(Entity, &GlobalTransform, &BasicTarget)>,
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
    for (entity, global_transform, target) in &target_query {
        let circle_pos = global_transform.translation();
        
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
