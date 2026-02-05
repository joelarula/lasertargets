use bevy::prelude::*;
// Removed unused import: use bevy_prototype_lyon::prelude::*;
use log::info;
use common::{path::{PathProvider, PathRenderable, UniversalPath}, scene::SceneData};

/// Extension trait to add gizmo drawing to UniversalPath
trait UniversalPathGizmos {
    fn draw_with_gizmos(&self, gizmos: &mut Gizmos, transform: &GlobalTransform, tolerance: f32);
}

impl UniversalPathGizmos for UniversalPath {
    fn draw_with_gizmos(&self, gizmos: &mut Gizmos, transform: &GlobalTransform, _tolerance: f32) {
        for segment in &self.segments {
            if segment.points.len() < 2 {
                continue;
            }
            
            // Draw lines between consecutive points
            for i in 0..segment.points.len() - 1 {
                let start_point = &segment.points[i];
                let end_point = &segment.points[i + 1];
                
                let start = transform.transform_point(Vec3::new(start_point.x, start_point.y, 0.0));
                let end = transform.transform_point(Vec3::new(end_point.x, end_point.y, 0.0));
                
                let color = Color::srgb(
                    start_point.r as f32 / 255.0,
                    start_point.g as f32 / 255.0,
                    start_point.b as f32 / 255.0,
                );
                
                gizmos.line(start, end, color);
            }
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct BasicTargetSystemSet;

#[derive(Component)]
pub struct BasicTarget {
    pub radius: f32,
    pub color: Color,
}

impl Default for BasicTarget {
    fn default() -> Self {
        Self {
            radius: 0.25,
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
    scene_data: Res<SceneData>,
    target_query: Query<(Entity, &GlobalTransform, &BasicTarget)>,
) {
    // Only check on mouse click
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    // Use prepared mouse world position from scene data
    let Some(mouse_world_pos) = scene_data.mouse_world_pos else {
        return;
    };

    // Check each target circle
    for (entity, global_transform, target) in &target_query {
        let circle_pos = global_transform.translation();
        
        // Check if click is within circle radius (already in world space)
        let distance_to_center = mouse_world_pos.distance(circle_pos);
        
        if distance_to_center < target.radius {
            info!("Clicked on basic target at {:?}, despawning", circle_pos);
            commands.entity(entity).despawn();
        }
    }
}
