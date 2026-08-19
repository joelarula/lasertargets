use bevy::prelude::*;
use common::path::{AbstractPathData, BroadcastScenePaths, UniversalPath};
use common::scene::SceneEntity;
use common::state::TerminalState;

/// Resource storing the current list of abstract minimal paths received from the server
#[derive(Resource, Default)]
pub struct TerminalScenePaths(pub Vec<AbstractPathData>);

/// Extension trait to add gizmo drawing to UniversalPath
trait UniversalPathGizmos {
    fn draw_with_gizmos(&self, gizmos: &mut Gizmos, transform: &Transform);
}

impl UniversalPathGizmos for UniversalPath {
    fn draw_with_gizmos(&self, gizmos: &mut Gizmos, transform: &Transform) {
        for segment in &self.segments {
            if segment.points.len() < 2 {
                continue;
            }
            
            for i in 0..segment.points.len() - 1 {
                let start_point = &segment.points[i];
                let end_point = &segment.points[i + 1];

                // Skip blanked laser moves (r=0, g=0, b=0)
                if (start_point.r == 0 && start_point.g == 0 && start_point.b == 0)
                    || (end_point.r == 0 && end_point.g == 0 && end_point.b == 0)
                {
                    continue;
                }
                
                let start = transform.transform_point(Vec3::new(start_point.x, start_point.y, 0.05));
                let end = transform.transform_point(Vec3::new(end_point.x, end_point.y, 0.05));
                
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

pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerminalScenePaths>()
            .add_message::<BroadcastScenePaths>()
            .add_systems(OnEnter(TerminalState::Connecting), clear_paths_on_disconnect)
            .add_systems(Update, handle_broadcast_scene_paths)
            .add_systems(Update, draw_paths);
    }
}

fn clear_paths_on_disconnect(mut scene_paths: ResMut<TerminalScenePaths>) {
    scene_paths.0.clear();
}

fn handle_broadcast_scene_paths(
    mut events: MessageReader<BroadcastScenePaths>,
    mut scene_paths: ResMut<TerminalScenePaths>,
) {
    for event in events.read() {
        scene_paths.0 = event.paths.clone();
    }
}

fn draw_paths(
    mut gizmos: Gizmos,
    scene_paths: Res<TerminalScenePaths>,
    scene_query: Query<&Transform, With<SceneEntity>>,
) {
    let scene_transform = scene_query.single().ok();

    for path_data in &scene_paths.0 {
        let transform = Transform::from_translation(path_data.position);
        if let Some(scene_transform) = scene_transform {
            let combined_transform = Transform::from_matrix(scene_transform.to_matrix() * transform.to_matrix());
            path_data.path.draw_with_gizmos(&mut gizmos, &combined_transform);
        } else {
            path_data.path.draw_with_gizmos(&mut gizmos, &transform);
        }
    }
}
