use bevy::prelude::*;
use common::path::{AbstractPathData, BroadcastScenePaths, PathRenderable, UniversalPath};
use common::scene::SceneEntity;

pub struct PathNetworkPlugin;

impl Plugin for PathNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BroadcastScenePaths>()
            .add_systems(PostUpdate, broadcast_combined_abstract_scene_paths);
    }
}

/// Aggregate all active abstract scene paths into a single BroadcastScenePaths payload every tick
fn broadcast_combined_abstract_scene_paths(
    paths_query: Query<
        (&UniversalPath, &Transform, Option<&ChildOf>),
        With<PathRenderable>,
    >,
    scene_query: Query<&Transform, With<SceneEntity>>,
    mut path_writer: MessageWriter<BroadcastScenePaths>,
) {
    let scene_transform = scene_query.single().ok();
    let mut abstract_paths = Vec::new();

    for (path, transform, parent) in paths_query.iter() {
        let position = if let Some(parent) = parent {
            if let Ok(p_transform) = scene_query.get(parent.parent()) {
                p_transform.transform_point(transform.translation)
            } else {
                transform.translation
            }
        } else if let Some(scene_transform) = scene_transform {
            scene_transform.transform_point(transform.translation)
        } else {
            transform.translation
        };

        abstract_paths.push(AbstractPathData {
            path: path.clone(),
            position,
        });
    }

    path_writer.write(BroadcastScenePaths {
        paths: abstract_paths,
    });
}
